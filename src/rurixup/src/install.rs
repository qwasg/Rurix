//! 原子分发与 content-tree 完整性(spec/release.md RXS-0135)。
//!
//! 编译器 / 运行时 / 标准库按**同一版号**作单一原子分发单元;分发 bundle 以
//! **content-tree 规范化 SHA-256**(复用 `rurix-pkg` RXS-0090 内容树规范化 /
//! RXS-0092 lock / RXS-0093 SHA-256)为完整性锚。**安装为全有或全无**:校验
//! 失败回滚,不留半装状态——`rurixup` 引导器据此原子安装与按版本切换。

use crate::bundle::{BundleManifest, Component, Partition};
use rurix_pkg::{content_tree, sha256};
use std::path::{Path, PathBuf};

/// 原子安装失败原因(工具层错误值,**非编译器 RX 段位码**,spec/release.md §3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    /// staged 内容树 SHA-256 与已发布(已签名)摘要不符 → 拒装并回滚。
    IntegrityMismatch {
        /// 已发布签名摘要(期望)。
        expected: String,
        /// staged 实测内容树摘要。
        actual: String,
    },
    /// 语言本体组件未同一版号(RXS-0135 原子分发判据)。
    VersionSkew,
    /// 单个组件磁盘字节 SHA-256 与 bundle 声明不符(RXS-0214 逐组件复核)。
    ComponentDigestMismatch {
        /// 组件干名。
        name: String,
        /// bundle 声明 digest(期望)。
        expected: String,
        /// staging 磁盘字节实测 digest。
        actual: String,
    },
    /// staged 组件在 bundle 清单中无对应条目(RXS-0214:源目录与清单不一致)。
    UnknownComponent(String),
    /// 真实文件系统 IO 失败(建目录 / 写字节 / 重命名 / 清 staging)。
    Io(String),
}

/// 原子安装回执(成功安装的不可变事实)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReceipt {
    /// 已安装版号。
    pub version: String,
    /// 已安装内容树 SHA-256(= 已发布签名摘要)。
    pub content_digest: String,
    /// 已提交组件数。
    pub component_count: usize,
}

/// 安装目标(承载已提交的分发内容;原子安装在此提交或回滚)。
#[derive(Debug, Default, Clone)]
pub struct InstallTarget {
    committed: Vec<(String, Vec<u8>)>,
    version: Option<String>,
}

impl InstallTarget {
    /// 空安装目标(尚未安装任何版本)。
    pub fn new() -> Self {
        InstallTarget {
            committed: Vec::new(),
            version: None,
        }
    }

    /// 当前已安装版号(`None` = 未安装)。
    pub fn installed_version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// 已提交组件数(回滚后保持安装前状态)。
    pub fn committed_len(&self) -> usize {
        self.committed.len()
    }

    /// 计算 staged 分发内容的规范化内容树 SHA-256(复用 rurix-pkg RXS-0093)。
    pub fn content_digest(staged: &[(String, Vec<u8>)]) -> String {
        content_tree::hash_entries(staged)
    }

    /// **原子安装**(RXS-0135):仅当 staged 内容树摘要 == 已发布签名摘要
    /// `expected_digest` 且 bundle 语言本体同一版号时,**全量提交**;否则
    /// **回滚**(安装目标保持安装前状态,不留半装)并返回 [`InstallError`]。
    pub fn atomic_install(
        &mut self,
        bundle: &BundleManifest,
        staged: &[(String, Vec<u8>)],
        expected_digest: &str,
    ) -> Result<InstallReceipt, InstallError> {
        if !bundle.language_core_versions_uniform() {
            // 回滚:不触碰已提交内容。
            return Err(InstallError::VersionSkew);
        }
        let actual = Self::content_digest(staged);
        if actual != expected_digest {
            // 完整性校验失败 → 回滚(self.committed / self.version 不变)。
            return Err(InstallError::IntegrityMismatch {
                expected: expected_digest.to_string(),
                actual,
            });
        }
        // 全有或全无:一次性替换已提交内容。
        self.committed = staged.to_vec();
        self.version = Some(bundle.rurix_version.clone());
        Ok(InstallReceipt {
            version: bundle.rurix_version.clone(),
            content_digest: actual,
            component_count: staged.len(),
        })
    }
}

// ————————————————— RXS-0214 真实 FS 物化与原子落盘 —————————————————
//
// 把已校验 bundle 内容树写入磁盘版本目录:staging 目录写入 → 逐组件 sha256 复核
// → tree_digest 双向复算(bundle 侧预算 == 磁盘侧 collect_dir 重哈希)→ **同卷单次
// rename** 原子提交到 toolchains/<version>/。任一校验失败 → 清 staging、不落
// toolchains\、不写注册表(零半装);重装幂等。全 safe(unsafe_code=deny),复用
// rurix-pkg content_tree/sha256(RXS-0090/0093)内核而非重写。

/// 真实 FS 物化回执(不可变事实:版号 + tree_digest + 磁盘落点 + 组件数)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializeReceipt {
    /// 已物化版号。
    pub version: String,
    /// 内容树 tree_digest(bundle 侧预算 == 磁盘侧复算的单一期望值)。
    pub tree_digest: String,
    /// 磁盘版本目录落点(`<RURIX_HOME>\toolchains\<version>`)。
    pub install_path: PathBuf,
    /// 已提交组件数。
    pub component_count: usize,
    /// 是否为幂等命中(目标已存在且 tree_digest 匹配,未重新物化)。
    pub idempotent_hit: bool,
}

/// 解析 `RURIX_HOME`(env `RURIX_HOME` 覆盖,默认 `%USERPROFILE%\.rurix`;测试缝 +
/// 多用户,RFC-0012 §4.1)。无 `USERPROFILE`(非 Windows / 剥离环境)时回退 `HOME`。
pub fn rurix_home() -> Result<PathBuf, InstallError> {
    if let Some(h) = std::env::var_os("RURIX_HOME") {
        return Ok(PathBuf::from(h));
    }
    let base = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| {
            InstallError::Io("无法解析 RURIX_HOME:USERPROFILE / HOME 均未设置".to_string())
        })?;
    Ok(PathBuf::from(base).join(".rurix"))
}

/// 组件干名 → `toolchains\<version>\` 下相对路径(确定性规则,RFC-0012 §4.1;
/// 不给 Component 加 path 字段——组件面仅数件,规则一屏可审):
/// - NVIDIA 再分发分区 → `nvidia/<name>`;
/// - 语言本体 `*.lib` → `bin/lib/<name>`(刻意对齐 driver.rs `current_exe().parent().join("lib")` 探测语义);
/// - 其余语言本体(`*.exe` 等)→ `bin/<name>`。
pub fn component_rel_path(comp: &Component) -> String {
    match comp.partition {
        Partition::NvidiaRedist => format!("nvidia/{}", comp.name),
        Partition::LanguageCore => {
            if comp.name.ends_with(".lib") {
                format!("bin/lib/{}", comp.name)
            } else {
                format!("bin/{}", comp.name)
            }
        }
    }
}

/// 从 bundle 清单预算 tree_digest(RFC-0012 §4.2):对每组件 `(rel_path, 声明 sha256)`
/// 做规范化内容树哈希——`content` 取 sha256 十六进制字节流,使**同一期望值**可从
/// bundle.json 预算、从磁盘经 collect_dir 重哈希复算(双向独立复算必相等)。
pub fn tree_digest_from_bundle(bundle: &BundleManifest) -> String {
    let entries: Vec<(String, Vec<u8>)> = bundle
        .components
        .iter()
        .map(|c| (component_rel_path(c), c.sha256.clone().into_bytes()))
        .collect();
    content_tree::hash_entries(&entries)
}

/// 从磁盘目录复算 tree_digest(RFC-0012 §4.2 磁盘侧):`collect_dir` 枚举实际
/// `(rel_path, 字节)` → 逐文件 sha256 → `(rel_path, sha256 十六进制字节流)` 再规范化
/// 哈希。与 [`tree_digest_from_bundle`] 对同一内容树产**逐字节一致**期望值。
pub fn tree_digest_from_dir(root: &Path) -> Result<String, InstallError> {
    let collected = content_tree::collect_dir(root)
        .map_err(|e| InstallError::Io(format!("collect_dir({}) 失败:{e}", root.display())))?;
    let entries: Vec<(String, Vec<u8>)> = collected
        .into_iter()
        .map(|(rel, bytes)| (rel, sha256::hex_digest(&bytes).into_bytes()))
        .collect();
    Ok(content_tree::hash_entries(&entries))
}

/// 生成 staging 目录名后缀(进程内唯一;非确定性但不入任何 digest——staging 名仅
/// 用于 rename 前隔离,断电孤儿按 `.staging-` 前缀例清)。
fn staging_nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{pid}-{seq}-{nanos}")
}

/// **真实 FS 物化**(RXS-0214):把 `staged`(组件干名 → 字节)按确定性相对路径规则
/// 写入 `<home>\tmp\.staging-*` → 逐组件 sha256 复核 == bundle 声明 → tree_digest
/// 双向复算相等 → **同卷单次 rename** 提交到 `<home>\toolchains\<version>`。任一失败
/// → 清 staging、不落 toolchains\、返回 [`InstallError`](调用方据此不写注册表)。
/// 重装幂等:目标已存在且 tree_digest 匹配 → 不重物化,返回 `idempotent_hit=true`。
pub fn materialize_to_disk(
    home: &Path,
    bundle: &BundleManifest,
    staged: &[(String, Vec<u8>)],
) -> Result<MaterializeReceipt, InstallError> {
    // 语言本体同一版号(RXS-0135 判据延续,先于任何落盘)。
    if !bundle.language_core_versions_uniform() {
        return Err(InstallError::VersionSkew);
    }
    let version = bundle.rurix_version.clone();
    let expected_tree = tree_digest_from_bundle(bundle);

    let target = home.join("toolchains").join(&version);

    // 幂等:目标已存在且 tree_digest 匹配 → 命中,不重物化(RFC-0012 §4.2 point 5)。
    if target.is_dir() {
        let on_disk = tree_digest_from_dir(&target)?;
        if on_disk == expected_tree {
            return Ok(MaterializeReceipt {
                version,
                tree_digest: expected_tree,
                install_path: target,
                component_count: bundle.components.len(),
                idempotent_hit: true,
            });
        }
        // 目标存在但内容漂移(损坏/异版覆盖)→ 走全量重物化(下方 rename 前替换)。
    }

    let staging_root = home.join("tmp");
    let staging = staging_root.join(format!(".staging-{version}-{}", staging_nonce()));
    // 建 staging(与 toolchains\ 同卷 ⇒ rename 原子);已存在残留先清。
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)
        .map_err(|e| InstallError::Io(format!("建 staging {} 失败:{e}", staging.display())))?;

    // 清 staging 的收尾闭包(任一失败路径调用,best-effort)。
    let cleanup = |st: &Path| {
        let _ = std::fs::remove_dir_all(st);
    };

    // 1+2. 逐组件落 staging + 逐文件 sha256 复核 == bundle 声明。
    for (name, bytes) in staged {
        let Some(comp) = bundle.components.iter().find(|c| &c.name == name) else {
            cleanup(&staging);
            return Err(InstallError::UnknownComponent(name.clone()));
        };
        let rel = component_rel_path(comp);
        let dest = staging.join(&rel);
        if let Some(parent) = dest.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            cleanup(&staging);
            return Err(InstallError::Io(format!(
                "建组件目录 {} 失败:{e}",
                parent.display()
            )));
        }
        if let Err(e) = std::fs::write(&dest, bytes) {
            cleanup(&staging);
            return Err(InstallError::Io(format!(
                "写组件 {} 失败:{e}",
                dest.display()
            )));
        }
        // 回读磁盘字节复核(不信内存 staged,读实际落盘内容)。
        let disk_bytes = match std::fs::read(&dest) {
            Ok(b) => b,
            Err(e) => {
                cleanup(&staging);
                return Err(InstallError::Io(format!(
                    "回读组件 {} 失败:{e}",
                    dest.display()
                )));
            }
        };
        let actual = sha256::hex_digest(&disk_bytes);
        if actual != comp.sha256 {
            cleanup(&staging);
            return Err(InstallError::ComponentDigestMismatch {
                name: name.clone(),
                expected: comp.sha256.clone(),
                actual,
            });
        }
    }

    // 3. tree_digest 磁盘侧复算 == bundle 侧预算(双向独立复算不变量)。
    let disk_tree = match tree_digest_from_dir(&staging) {
        Ok(d) => d,
        Err(e) => {
            cleanup(&staging);
            return Err(e);
        }
    };
    if disk_tree != expected_tree {
        cleanup(&staging);
        return Err(InstallError::IntegrityMismatch {
            expected: expected_tree,
            actual: disk_tree,
        });
    }

    // 4. 提交 = 同卷单次目录 rename(staging → toolchains\<version>)。
    if let Some(parent) = target.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        cleanup(&staging);
        return Err(InstallError::Io(format!(
            "建 toolchains 目录 {} 失败:{e}",
            parent.display()
        )));
    }
    // Windows rename 拒目标已存在:漂移/损坏场景先移除旧目录(修复)。
    if target.exists()
        && let Err(e) = std::fs::remove_dir_all(&target)
    {
        cleanup(&staging);
        return Err(InstallError::Io(format!(
            "移除旧版本目录 {} 失败:{e}",
            target.display()
        )));
    }
    if let Err(e) = std::fs::rename(&staging, &target) {
        cleanup(&staging);
        return Err(InstallError::Io(format!(
            "提交 rename {} → {} 失败:{e}",
            staging.display(),
            target.display()
        )));
    }

    Ok(MaterializeReceipt {
        version,
        tree_digest: expected_tree,
        install_path: target,
        component_count: bundle.components.len(),
        idempotent_hit: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{Component, Partition};

    fn bundle_v(ver: &str) -> BundleManifest {
        let mut b = BundleManifest::new(ver);
        b.push(Component {
            name: "rurixc.exe".to_string(),
            version: ver.to_string(),
            license: "Apache-2.0".to_string(),
            partition: Partition::LanguageCore,
            sha256: "00".repeat(32),
        });
        b
    }

    //@ spec: RXS-0135
    // 原子安装:staged 内容树摘要 == 已发布签名摘要 → 全量提交;篡改任一字节
    // → 摘要不符 → 拒装并回滚(安装目标保持安装前状态,不留半装)。
    #[test]
    fn atomic_install_verifies_content_tree() {
        let bundle = bundle_v("0.1.0");
        let staged = vec![
            ("bin/rurixc.exe".to_string(), b"MZ-rurixc-payload".to_vec()),
            ("lib/std.rlib".to_string(), b"std-archive".to_vec()),
        ];
        let published = InstallTarget::content_digest(&staged);

        // 绿:摘要匹配 → 提交全部组件。
        let mut target = InstallTarget::new();
        let receipt = target
            .atomic_install(&bundle, &staged, &published)
            .expect("matching digest installs atomically");
        assert_eq!(receipt.version, "0.1.0");
        assert_eq!(receipt.content_digest, published);
        assert_eq!(receipt.component_count, 2);
        assert_eq!(target.committed_len(), 2);
        assert_eq!(target.installed_version(), Some("0.1.0"));

        // 红:篡改一个字节 → 内容树摘要变化 → 拒装并回滚。
        let mut tampered = staged.clone();
        tampered[0].1[0] ^= 0xFF;
        let mut fresh = InstallTarget::new();
        let err = fresh
            .atomic_install(&bundle, &tampered, &published)
            .expect_err("tampered content must be rejected");
        match err {
            InstallError::IntegrityMismatch { expected, actual } => {
                assert_eq!(expected, published);
                assert_ne!(actual, published);
            }
            other => panic!("expected IntegrityMismatch, got {other:?}"),
        }
        // 回滚:未提交任何内容(全有或全无)。
        assert_eq!(fresh.committed_len(), 0);
        assert_eq!(fresh.installed_version(), None);
    }

    //@ spec: RXS-0135
    // 内容树摘要不依赖 staged 切片顺序(规范化排序,复用 rurix-pkg RXS-0090/0093)。
    #[test]
    fn content_digest_is_order_independent() {
        let a = vec![
            ("z.bin".to_string(), b"zzz".to_vec()),
            ("a.bin".to_string(), b"aaa".to_vec()),
        ];
        let b = vec![
            ("a.bin".to_string(), b"aaa".to_vec()),
            ("z.bin".to_string(), b"zzz".to_vec()),
        ];
        assert_eq!(
            InstallTarget::content_digest(&a),
            InstallTarget::content_digest(&b)
        );
    }

    // ————————————————— RXS-0214 真实 FS 物化单测 —————————————————

    /// 唯一临时 home 目录(测试缝;测试尾清理)。
    fn temp_home(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!(
            "rurixup-ea11a-{tag}-{}-{}",
            std::process::id(),
            nanos
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("建临时 home");
        p
    }

    /// 由真实字节建 bundle(digest 真算)+ staged 组件对。
    fn bundle_with(
        comps: &[(&str, Partition, &[u8])],
        ver: &str,
    ) -> (BundleManifest, Vec<(String, Vec<u8>)>) {
        let mut b = BundleManifest::new(ver);
        let mut staged = Vec::new();
        for (name, part, bytes) in comps {
            b.push(Component {
                name: name.to_string(),
                version: ver.to_string(),
                license: "Apache-2.0".to_string(),
                partition: *part,
                sha256: sha256::hex_digest(bytes),
            });
            staged.push((name.to_string(), bytes.to_vec()));
        }
        (b, staged)
    }

    //@ spec: RXS-0214
    // 组件干名 → 相对路径确定性规则:*.exe→bin/、*.lib→bin/lib/、NvidiaRedist→nvidia/。
    #[test]
    fn component_rel_path_deterministic_rule() {
        let exe = Component {
            name: "rx.exe".into(),
            version: "1".into(),
            license: "L".into(),
            partition: Partition::LanguageCore,
            sha256: "00".into(),
        };
        let lib = Component {
            name: "rurix_rt_cabi.lib".into(),
            version: "1".into(),
            license: "L".into(),
            partition: Partition::LanguageCore,
            sha256: "00".into(),
        };
        let nv = Component {
            name: "libdevice.10.bc".into(),
            version: "1".into(),
            license: "L".into(),
            partition: Partition::NvidiaRedist,
            sha256: "00".into(),
        };
        assert_eq!(component_rel_path(&exe), "bin/rx.exe");
        assert_eq!(component_rel_path(&lib), "bin/lib/rurix_rt_cabi.lib");
        assert_eq!(component_rel_path(&nv), "nvidia/libdevice.10.bc");
    }

    //@ spec: RXS-0214
    // tree_digest 双向独立复算相等:bundle 侧预算 == 磁盘侧 collect_dir 重哈希。
    #[test]
    fn materialize_green_bidirectional_tree_digest_and_bytes() {
        let home = temp_home("green");
        let (bundle, staged) = bundle_with(
            &[
                ("rx.exe", Partition::LanguageCore, b"MZ-rx-payload"),
                (
                    "rurix_rt_cabi.lib",
                    Partition::LanguageCore,
                    b"!<arch>\nlib-bytes",
                ),
            ],
            "1.0.0",
        );
        let receipt = materialize_to_disk(&home, &bundle, &staged).expect("物化成功");
        assert!(!receipt.idempotent_hit);
        assert_eq!(receipt.component_count, 2);
        // 磁盘树在 + 逐字节 == 源。
        let rx = home.join("toolchains/1.0.0/bin/rx.exe");
        let lib = home.join("toolchains/1.0.0/bin/lib/rurix_rt_cabi.lib");
        assert!(rx.is_file() && lib.is_file());
        assert_eq!(std::fs::read(&rx).unwrap(), b"MZ-rx-payload");
        assert_eq!(std::fs::read(&lib).unwrap(), b"!<arch>\nlib-bytes");
        // 双向复算相等。
        assert_eq!(receipt.tree_digest, tree_digest_from_bundle(&bundle));
        assert_eq!(
            receipt.tree_digest,
            tree_digest_from_dir(&receipt.install_path).unwrap()
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    //@ spec: RXS-0214
    // 重装幂等:同 bundle 再物化 → idempotent_hit=true,不重写;tree_digest 不变。
    #[test]
    fn materialize_is_idempotent() {
        let home = temp_home("idem");
        let (bundle, staged) =
            bundle_with(&[("rx.exe", Partition::LanguageCore, b"payload")], "2.0.0");
        let r1 = materialize_to_disk(&home, &bundle, &staged).expect("首装");
        assert!(!r1.idempotent_hit);
        let r2 = materialize_to_disk(&home, &bundle, &staged).expect("重装幂等");
        assert!(r2.idempotent_hit);
        assert_eq!(r1.tree_digest, r2.tree_digest);
        let _ = std::fs::remove_dir_all(&home);
    }

    //@ spec: RXS-0214
    // 篡改一组件字节(digest != bundle 声明)→ 逐组件 sha256 复核拒 → 零残留、目标不诞生。
    #[test]
    fn materialize_rolls_back_on_component_tamper_zero_residue() {
        let home = temp_home("tamper");
        let (bundle, mut staged) = bundle_with(
            &[("rx.exe", Partition::LanguageCore, b"clean-bytes")],
            "3.0.0",
        );
        // 篡改 staged 字节但保持 bundle 声明的原 digest → 磁盘字节 sha256 失配。
        staged[0].1 = b"TAMPERED-bytes".to_vec();
        let err = materialize_to_disk(&home, &bundle, &staged).expect_err("篡改组件须拒");
        assert!(matches!(err, InstallError::ComponentDigestMismatch { .. }));
        // 零半装:版本目录不诞生。
        assert!(!home.join("toolchains/3.0.0").exists());
        // staging 已清(tmp 下无 .staging- 残留)。
        if let Ok(rd) = std::fs::read_dir(home.join("tmp")) {
            for e in rd.flatten() {
                assert!(
                    !e.file_name().to_string_lossy().starts_with(".staging-"),
                    "staging 未清:{:?}",
                    e.file_name()
                );
            }
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    //@ spec: RXS-0214
    // 语言本体版号偏移 → VersionSkew,先于任何落盘(目标不诞生)。
    #[test]
    fn materialize_rejects_version_skew_before_disk() {
        let home = temp_home("skew");
        let mut bundle = BundleManifest::new("1.0.0");
        bundle.push(Component {
            name: "rx.exe".into(),
            version: "0.9.9".into(),
            license: "L".into(),
            partition: Partition::LanguageCore,
            sha256: sha256::hex_digest(b"x"),
        });
        let staged = vec![("rx.exe".to_string(), b"x".to_vec())];
        let err = materialize_to_disk(&home, &bundle, &staged).expect_err("版号偏移须拒");
        assert_eq!(err, InstallError::VersionSkew);
        assert!(!home.join("toolchains/1.0.0").exists());
        let _ = std::fs::remove_dir_all(&home);
    }
}
