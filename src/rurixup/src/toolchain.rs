//! 本地工具链版本注册 + stable channel 消费(spec/release.md RXS-0187 ~ RXS-0188;
//! post-V1,Mini-RFC/MR-0009)。
//!
//! rurixup 工具链管理前端**首切片**:从 `rurixup release` 产出的 stable channel
//! 清单([`crate::channel::ChannelManifest`],MR-0008)+ `bundle.json` **消费**
//! stable channel,把版本**注册**进确定性工具链注册表(`toolchains.json`),支持
//! **多版本共存 + 默认版本切换**。
//!
//! **复用**:安装完整性判据复用 [`crate::install`] content-tree SHA-256 内核
//! (RXS-0135);channel 一致性判据复用 [`crate::channel::consistent`](RXS-0186);
//! 内容寻址校验复用 `rurix_pkg::sha256`(RXS-0093)。**纯 host、纯确定性、零网络
//! 端点、零真实 FS 物化**(`unsafe_code=deny`)。
//!
//! **defer(RD-025)**:真实文件系统物化(磁盘版本目录、PATH/junction 活跃切换)与
//! 网络拉取(URL 下载 channel/bundle)属真实 IO / 安全包络 / 网络端点面,不在本切片
//! 落笔;届时按 10 §3 判档。

use crate::bundle::BundleManifest;
use crate::channel::{self, ChannelManifest};
use crate::json_escape;

/// 工具链前端错误(工具层 Result,**非编译器 RX 段位码**,spec/release.md §3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainError {
    /// channel 清单与 bundle 不一致(channel 非法 / 版号不符 / 组件全集不对应,RXS-0186)。
    ManifestInconsistent,
    /// channel 清单记录的 bundle digest 与实测 `sha256(bundle_json)` 失配(内容寻址校验,RXS-0135/0187)。
    DigestMismatch {
        /// channel 清单声明的 bundle digest。
        declared: String,
        /// 实测 bundle.json 字节流 digest。
        actual: String,
    },
    /// `set_default` 指向未注册版号(RXS-0187)。
    UnknownVersion(String),
}

/// 单个已注册工具链版本(不可变事实:版号 + 已校验 bundle 内容寻址 digest;
/// **schema v2** 追加磁盘落点 `install_path` + 内容树 `tree_digest`,RXS-0214)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledToolchain {
    /// 已注册版号。
    pub version: String,
    /// 已校验的 bundle 清单 content digest(= channel 清单 `bundle_manifest_sha256`)。
    pub content_digest: String,
    /// 磁盘版本目录落点(`<RURIX_HOME>\toolchains\<version>`);`None` = **registered-only**
    /// (纯账面注册,无真实 FS 物化——v1 旧条目读入 / RXS-0188 账面 install 路径)。
    pub install_path: Option<String>,
    /// 内容树 tree_digest(真实物化时记录,RXS-0214);`None` = registered-only。
    pub tree_digest: Option<String>,
}

impl InstalledToolchain {
    /// 是否为 registered-only(纯账面,无真实磁盘物化)。
    pub fn is_registered_only(&self) -> bool {
        self.install_path.is_none()
    }
}

/// 工具链注册表(纯确定性状态;序列化为 `toolchains.json`)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolchainRegistry {
    installed: Vec<InstalledToolchain>,
    default: Option<String>,
}

impl ToolchainRegistry {
    /// 空注册表(尚未注册任何版本)。
    pub fn new() -> Self {
        ToolchainRegistry::default()
    }

    /// **消费 stable channel 并注册版本**(RXS-0188):校验 channel 清单与 bundle
    /// 一致(RXS-0186 `consistent`)+ `bundle_manifest_sha256` == 实测
    /// `sha256(bundle_json)`(内容寻址,RXS-0135/0093 口径);通过则注册该版本
    /// (幂等:同一 `(version, digest)` 重复 install = no-op)。首个注册版本自动
    /// 成为 default。**全有或全无**:校验失败不注册(对齐 RXS-0135 原子性)。
    pub fn install(
        &mut self,
        manifest: &ChannelManifest,
        bundle: &BundleManifest,
        bundle_json: &str,
    ) -> Result<String, ToolchainError> {
        if !channel::consistent(bundle, manifest) {
            return Err(ToolchainError::ManifestInconsistent);
        }
        let actual = rurix_pkg::sha256::hex_digest(bundle_json.as_bytes());
        if manifest.bundle_manifest_sha256 != actual {
            return Err(ToolchainError::DigestMismatch {
                declared: manifest.bundle_manifest_sha256.clone(),
                actual,
            });
        }
        let entry = InstalledToolchain {
            version: manifest.rurix_version.clone(),
            content_digest: actual,
            install_path: None,
            tree_digest: None,
        };
        // 幂等:已注册同 (version, digest) → no-op。
        if !self.installed.contains(&entry) {
            // 同版号不同 digest 视为重装覆盖(替换该版号条目,保持内容寻址唯一)。
            self.installed.retain(|t| t.version != entry.version);
            self.installed.push(entry.clone());
            self.installed.sort_by(|a, b| a.version.cmp(&b.version));
        }
        // 首个注册版本自动成为 default。
        if self.default.is_none() {
            self.default = Some(entry.version.clone());
        }
        Ok(entry.version)
    }

    /// **登记真实物化版本**(RXS-0214):在账面校验(RXS-0188)之外记录磁盘落点
    /// `install_path` 与内容树 `tree_digest`(schema v2)。幂等:同 `(version, content_digest,
    /// install_path, tree_digest)` 重复登记 = no-op;同版号异内容 = 重装覆盖。首个登记
    /// 版本自动成为 default。调用序:先 [`install`](账面内容寻址校验)后本方法(物化落点),
    /// 或直接以已校验事实登记。
    pub fn register_materialized(
        &mut self,
        version: &str,
        content_digest: &str,
        install_path: &str,
        tree_digest: &str,
    ) {
        let entry = InstalledToolchain {
            version: version.to_string(),
            content_digest: content_digest.to_string(),
            install_path: Some(install_path.to_string()),
            tree_digest: Some(tree_digest.to_string()),
        };
        if !self.installed.contains(&entry) {
            self.installed.retain(|t| t.version != entry.version);
            self.installed.push(entry.clone());
            self.installed.sort_by(|a, b| a.version.cmp(&b.version));
        }
        if self.default.is_none() {
            self.default = Some(entry.version.clone());
        }
    }

    /// 取指定版号条目(只读)。
    pub fn get(&self, version: &str) -> Option<&InstalledToolchain> {
        self.installed.iter().find(|t| t.version == version)
    }

    /// 设默认版本(RXS-0187):`version` 须已注册,否则 [`ToolchainError::UnknownVersion`]。
    pub fn set_default(&mut self, version: &str) -> Result<(), ToolchainError> {
        if self.installed.iter().any(|t| t.version == version) {
            self.default = Some(version.to_string());
            Ok(())
        } else {
            Err(ToolchainError::UnknownVersion(version.to_string()))
        }
    }

    /// 已注册版本(版号字典序)。
    pub fn list(&self) -> &[InstalledToolchain] {
        &self.installed
    }

    /// 当前默认版号(`None` = 无注册版本)。
    pub fn default_version(&self) -> Option<&str> {
        self.default.as_deref()
    }

    /// 确定性 JSON 序列化(RXS-0187):版号字典序,**不含时间戳**——同一操作序列
    /// 产逐字节一致字节流(镜像 RXS-0138/0185 确定性纪律)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str("  \"schema_version\": 2,\n");
        match &self.default {
            Some(v) => s.push_str(&format!("  \"default\": \"{}\",\n", json_escape(v))),
            None => s.push_str("  \"default\": null,\n"),
        }
        s.push_str("  \"installed\": [\n");
        for (i, t) in self.installed.iter().enumerate() {
            let comma = if i + 1 < self.installed.len() {
                ","
            } else {
                ""
            };
            s.push_str("    {\n");
            s.push_str(&format!(
                "      \"version\": \"{}\",\n",
                json_escape(&t.version)
            ));
            s.push_str(&format!(
                "      \"content_digest\": \"{}\",\n",
                json_escape(&t.content_digest)
            ));
            // schema v2:install_path / tree_digest;registered-only(v1 / 账面)= null。
            match &t.install_path {
                Some(p) => s.push_str(&format!(
                    "      \"install_path\": \"{}\",\n",
                    json_escape(p)
                )),
                None => s.push_str("      \"install_path\": null,\n"),
            }
            match &t.tree_digest {
                Some(d) => s.push_str(&format!("      \"tree_digest\": \"{}\"\n", json_escape(d))),
                None => s.push_str("      \"tree_digest\": null\n"),
            }
            s.push_str(&format!("    }}{comma}\n"));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// 从 `toolchains.json` 解析(确定性 round-trip:`from_json(to_json(r)) == r`)。
    /// 手写极简解析(零外部依赖,仅识别本模块 `to_json` 产出的规范形态)。
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut reg = ToolchainRegistry::new();
        let mut default: Option<String> = None;
        let mut cur_version: Option<String> = None;
        let mut cur_digest: Option<String> = None;
        let mut cur_install_path: Option<String> = None;
        // 可空标量字段解析:`"key": null` → None;`"key": "v"` → Some(v)。
        let opt_field = |raw: &str| -> Result<Option<String>, String> {
            let v = raw.trim().trim_end_matches(',').trim();
            if v == "null" {
                Ok(None)
            } else {
                Ok(Some(unquote(v)?))
            }
        };
        for raw in text.lines() {
            let line = raw.trim();
            if let Some(rest) = line.strip_prefix("\"default\":") {
                let v = rest.trim().trim_end_matches(',').trim();
                if v != "null" {
                    default = Some(unquote(v)?);
                }
            } else if let Some(rest) = line.strip_prefix("\"version\":") {
                // 若上一条目为 v1 形态(有 version+content_digest 但无 tree_digest 收束行),
                // 遇下一 version 前先 flush(v1 多条目兼容读入)。
                if let (Some(v), Some(d)) = (cur_version.take(), cur_digest.take()) {
                    reg.installed.push(InstalledToolchain {
                        version: v,
                        content_digest: d,
                        install_path: cur_install_path.take(),
                        tree_digest: None,
                    });
                }
                cur_version = Some(unquote(rest.trim().trim_end_matches(','))?);
                cur_digest = None;
                cur_install_path = None;
            } else if let Some(rest) = line.strip_prefix("\"content_digest\":") {
                cur_digest = Some(unquote(rest.trim().trim_end_matches(','))?);
            } else if let Some(rest) = line.strip_prefix("\"install_path\":") {
                // v2 字段;v1 旧条目无此行 → 保持 None(registered-only 兼容读入)。
                cur_install_path = opt_field(rest)?;
            } else if let Some(rest) = line.strip_prefix("\"tree_digest\":") {
                // 条目末字段(v2):此处收束一个 InstalledToolchain。
                let tree_digest = opt_field(rest)?;
                let version = cur_version
                    .take()
                    .ok_or_else(|| "tree_digest 无匹配 version".to_string())?;
                let content_digest = cur_digest
                    .take()
                    .ok_or_else(|| "tree_digest 无匹配 content_digest".to_string())?;
                reg.installed.push(InstalledToolchain {
                    version,
                    content_digest,
                    install_path: cur_install_path.take(),
                    tree_digest,
                });
            }
        }
        // v1 兼容:content_digest 为条目末字段(无 install_path/tree_digest 行),收束遗留条目。
        if let (Some(version), Some(content_digest)) = (cur_version.take(), cur_digest.take()) {
            reg.installed.push(InstalledToolchain {
                version,
                content_digest,
                install_path: cur_install_path.take(),
                tree_digest: None,
            });
        }
        reg.installed.sort_by(|a, b| a.version.cmp(&b.version));
        // default 须在已注册集合内(否则损坏)。
        if let Some(ref d) = default
            && !reg.installed.iter().any(|t| &t.version == d)
        {
            return Err(format!("toolchains.json default `{d}` 未注册(状态损坏)"));
        }
        reg.default = default;
        Ok(reg)
    }
}

/// 解析 JSON 字符串字面量(识别 `"..."` 并反转义 `json_escape` 产出的转义序列
/// `\" \\ \n \r \t \uXXXX`)。版号/digest 为纯 ASCII 无转义,原样返回;install_path
/// 含 Windows 反斜杠 `\`(json_escape → `\\`),此处还原以保 round-trip(RXS-0214)。
fn unquote(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') {
        return Err(format!("非法 JSON 字符串字面量:{s}"));
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('/') => out.push('/'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = (&mut chars).take(4).collect();
                let cp =
                    u32::from_str_radix(&hex, 16).map_err(|_| format!("非法 \\u 转义:\\u{hex}"))?;
                out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
            }
            Some(other) => return Err(format!("未知转义序列 \\{other}")),
            None => return Err("字符串以孤立反斜杠结尾".to_string()),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{Component, Partition};

    fn bundle(ver: &str) -> BundleManifest {
        let mut b = BundleManifest::new(ver);
        b.push(Component {
            name: "rurixup.exe".to_string(),
            version: ver.to_string(),
            license: "Apache-2.0".to_string(),
            partition: Partition::LanguageCore,
            sha256: "aa".repeat(32),
        });
        b
    }

    fn manifest_for(b: &BundleManifest) -> (ChannelManifest, String) {
        let bj = b.to_json();
        let m = channel::generate(b, "stable", &bj).expect("stable 合法");
        (m, bj)
    }

    //@ spec: RXS-0188
    // 消费 stable channel:一致 + digest 匹配 → 注册;篡改 bundle(digest 失配)→ 拒;
    // channel 与 bundle 不一致 → 拒(全有或全无,不注册)。
    #[test]
    fn install_consumes_stable_channel_with_verification() {
        let b = bundle("1.0.0");
        let (m, bj) = manifest_for(&b);
        let mut reg = ToolchainRegistry::new();

        // green:一致 + digest 匹配 → 注册 + 首个成 default。
        let v = reg.install(&m, &b, &bj).expect("verified install");
        assert_eq!(v, "1.0.0");
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.default_version(), Some("1.0.0"));

        // red:篡改 bundle_json(digest 失配)→ 拒,不注册。
        let mut reg2 = ToolchainRegistry::new();
        let err = reg2
            .install(&m, &b, &format!("{bj} "))
            .expect_err("tampered bundle_json must be rejected");
        assert!(matches!(err, ToolchainError::DigestMismatch { .. }));
        assert_eq!(reg2.list().len(), 0);

        // red:channel 与 bundle 不一致(版号漂移)→ 拒。
        let b2 = bundle("2.0.0");
        let err = reg2
            .install(&m, &b2, &b2.to_json())
            .expect_err("inconsistent manifest must be rejected");
        assert_eq!(err, ToolchainError::ManifestInconsistent);
        assert_eq!(reg2.list().len(), 0);
    }

    //@ spec: RXS-0187
    // 多版本注册幂等 + 默认切换 + 未注册版号拒 + 确定性序列化 round-trip。
    #[test]
    fn multi_version_register_default_and_determinism() {
        let mut reg = ToolchainRegistry::new();
        for ver in ["1.0.0", "1.1.0"] {
            let b = bundle(ver);
            let (m, bj) = manifest_for(&b);
            reg.install(&m, &b, &bj).expect("install");
        }
        assert_eq!(reg.list().len(), 2);
        // 首个(1.0.0)仍为 default。
        assert_eq!(reg.default_version(), Some("1.0.0"));

        // 幂等:re-install 1.0.0 = no-op(不重复入表)。
        let b = bundle("1.0.0");
        let (m, bj) = manifest_for(&b);
        reg.install(&m, &b, &bj).expect("idempotent");
        assert_eq!(reg.list().len(), 2);

        // 默认切换:已注册版号 OK,未注册版号拒。
        reg.set_default("1.1.0").expect("known version");
        assert_eq!(reg.default_version(), Some("1.1.0"));
        assert_eq!(
            reg.set_default("9.9.9"),
            Err(ToolchainError::UnknownVersion("9.9.9".to_string()))
        );

        // 确定性:两次序列化逐字节一致 + round-trip 保真。
        assert_eq!(reg.to_json(), reg.to_json());
        let parsed = ToolchainRegistry::from_json(&reg.to_json()).expect("round-trip");
        assert_eq!(parsed, reg);
        // 序列化不含时间戳。
        assert!(!reg.to_json().to_lowercase().contains("timestamp"));
    }

    //@ spec: RXS-0214
    // 注册表 schema v2:register_materialized 记录 install_path/tree_digest;确定性
    // round-trip 保真;v1 旧条目(无 install_path 行)读入标 registered-only 不丢弃。
    #[test]
    fn registry_v2_materialized_roundtrip_and_v1_compat() {
        let mut reg = ToolchainRegistry::new();
        reg.register_materialized(
            "1.0.0",
            &"aa".repeat(32),
            "C:\\home\\toolchains\\1.0.0",
            &"bb".repeat(32),
        );
        reg.register_materialized(
            "1.1.0",
            &"cc".repeat(32),
            "C:\\home\\toolchains\\1.1.0",
            &"dd".repeat(32),
        );
        // schema v2 + install_path/tree_digest 落 JSON。
        let json = reg.to_json();
        assert!(json.contains("\"schema_version\": 2"));
        assert!(json.contains("\"install_path\": \"C:\\\\home\\\\toolchains\\\\1.0.0\""));
        assert!(json.contains("\"tree_digest\""));
        // 首装成 default;get + is_registered_only。
        assert_eq!(reg.default_version(), Some("1.0.0"));
        let e = reg.get("1.1.0").expect("已注册");
        assert!(!e.is_registered_only());
        assert_eq!(
            e.install_path.as_deref(),
            Some("C:\\home\\toolchains\\1.1.0")
        );
        // 确定性 round-trip 保真。
        assert_eq!(reg.to_json(), reg.to_json());
        assert_eq!(
            ToolchainRegistry::from_json(&json).expect("round-trip"),
            reg
        );

        // 幂等:同事实重登记 = no-op。
        reg.register_materialized(
            "1.0.0",
            &"aa".repeat(32),
            "C:\\home\\toolchains\\1.0.0",
            &"bb".repeat(32),
        );
        assert_eq!(reg.list().len(), 2);

        // v1 兼容读入:旧格式(schema_version 1,无 install_path/tree_digest 行)→ registered-only。
        let v1 = concat!(
            "{\n",
            "  \"schema_version\": 1,\n",
            "  \"default\": \"0.9.0\",\n",
            "  \"installed\": [\n",
            "    {\n",
            "      \"version\": \"0.9.0\",\n",
            "      \"content_digest\": \"ee\"\n",
            "    },\n",
            "    {\n",
            "      \"version\": \"0.9.1\",\n",
            "      \"content_digest\": \"ff\"\n",
            "    }\n",
            "  ]\n}\n"
        );
        let parsed = ToolchainRegistry::from_json(v1).expect("v1 读入");
        assert_eq!(parsed.list().len(), 2, "v1 多条目不丢弃");
        assert!(parsed.get("0.9.0").unwrap().is_registered_only());
        assert!(parsed.get("0.9.1").unwrap().is_registered_only());
        assert_eq!(parsed.default_version(), Some("0.9.0"));
    }
}
