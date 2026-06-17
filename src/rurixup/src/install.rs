//! 原子分发与 content-tree 完整性(spec/release.md RXS-0135)。
//!
//! 编译器 / 运行时 / 标准库按**同一版号**作单一原子分发单元;分发 bundle 以
//! **content-tree 规范化 SHA-256**(复用 `rurix-pkg` RXS-0090 内容树规范化 /
//! RXS-0092 lock / RXS-0093 SHA-256)为完整性锚。**安装为全有或全无**:校验
//! 失败回滚,不留半装状态——`rurixup` 引导器据此原子安装与按版本切换。

use crate::bundle::BundleManifest;
use rurix_pkg::content_tree;

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
}
