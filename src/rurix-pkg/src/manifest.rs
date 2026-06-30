//! rurix.toml 清单模型与解析(RXS-0089 清单格式 / RXS-0090 三来源)。
//!
//! 声明式无 build.rs(09 §7.1):`[package].build` 缺省或唯一合法值 `"declarative"`;
//! 任何其他值 → [`PkgError::ManifestInvalid`](crate::PkgError)(RX7005)。

use std::collections::BTreeMap;

use crate::error::{PkgError, PkgResult};
use crate::toml::{self, Value};

/// edition 合法值集合(首期,冻结于 RFC-0008 §4.2;新增 edition 经后续 Full RFC,
/// 不在本里程碑扩展)。spec/edition.md RXS-0178 L1 与本常量一字对齐。
pub const VALID_EDITIONS: &[&str] = &["2026"];

/// 语义版本边界声明(RXS-0177~0180;RFC-0008)。首个 edition `"2026"` 定位为
/// **机制锚点**:首期 edition-gated 行为差异 = 空集(`"2026"` 与无 edition 声明
/// 行为完全一致,RFC-0008 §9 Q-Scope),仅建立"语言面有 edition 边界、未来破坏性
/// 变更经 edition 隔离"的机制基座。纯编译期/host 声明语义,不触 🔒 禁区。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Edition {
    /// 首个 edition `"2026"`(缺省;RFC-0008 §9 Q-Name / Q-Default)。
    #[default]
    Edition2026,
}

/// edition 解析失败(RXS-0179;未知/不匹配 edition,strict-only,无 fallback,P-01)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditionError {
    /// 字符串值不在合法集合 `{ "2026" }`(映射 RX7020,无 fallback,不回退缺省)。
    Unknown(String),
}

impl Edition {
    /// 首个 edition(缺省 edition,RXS-0177 缺省语义:清单缺 `edition` 键时取此值)。
    pub const FIRST: Edition = Edition::Edition2026;

    /// edition 字符串 → 内部表示(RXS-0178,确定性纯函数,无 I/O / 无环境依赖)。
    /// 合法集合外 → `EditionError::Unknown`(strict-only,无 fallback,P-01)。
    pub fn parse(s: &str) -> Result<Edition, EditionError> {
        match s {
            "2026" => Ok(Edition::Edition2026),
            other => Err(EditionError::Unknown(other.to_owned())),
        }
    }

    /// edition 的规范字符串(stable 面版本锚,RXS-0180 L1;进 stable 快照内容)。
    pub fn as_str(&self) -> &'static str {
        match self {
            Edition::Edition2026 => "2026",
        }
    }

    /// edition-gated 行为分发锚点(RXS-0177 / RFC-0008 §4.5)。首期 edition-gated
    /// 行为差异 = 空集 → 任意两 edition 间恒无行为差异(返回 `false`);未来 edition
    /// 在此接入差异分发,而非散落 ad-hoc 版本判断(机制锚点)。
    pub fn behavior_differs(&self, _other: Edition) -> bool {
        // 首期仅 Edition2026,edition-gated 行为差异 = 空集(RFC-0008 §9 Q-Scope)。
        false
    }
}

/// 语义化三段版本(major.minor.patch)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn parse(s: &str) -> Result<Version, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("版本须为 major.minor.patch,实得 {s:?}"));
        }
        let parse_part = |p: &str| -> Result<u64, String> {
            p.parse::<u64>()
                .map_err(|_| format!("版本段非非负整数 {p:?}"))
        };
        Ok(Version {
            major: parse_part(parts[0])?,
            minor: parse_part(parts[1])?,
            patch: parse_part(parts[2])?,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 依赖来源(三选一,互斥;RXS-0090)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// 本地路径源(相对清单目录)。
    Path(String),
    /// git 源(rev 精确提交,供逐字节复现)。
    Git { url: String, rev: String },
    /// 归档源(sha256 内容指纹)。
    Archive { url: String, sha256: String },
}

impl Source {
    /// rurix.lock 稳定编码(RXS-0092):`path:<rel>` / `git:<url>#<rev>` /
    /// `archive:<url>#<sha256>`。
    pub fn locator(&self) -> String {
        match self {
            Source::Path(p) => format!("path:{p}"),
            Source::Git { url, rev } => format!("git:{url}#{rev}"),
            Source::Archive { url, sha256 } => format!("archive:{url}#{sha256}"),
        }
    }

    /// 是否为离线可解析的本地来源(RXS-0094:--offline 仅许 path + vendor 缓存)。
    pub fn is_local(&self) -> bool {
        matches!(self, Source::Path(_))
    }
}

/// 单个依赖(来源 + 选定 feature)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub source: Source,
    pub features: Vec<String>,
    pub default_features: bool,
}

/// rurix.toml 清单。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub version: Version,
    /// 语义版本边界(RXS-0177;缺省 = 首个 edition `Edition::FIRST`,向后兼容)。
    pub edition: Edition,
    pub dependencies: BTreeMap<String, Dependency>,
    /// feature → 启用项(其他 feature 或 `<dep>` / `<dep>/<feat>`)。
    pub features: BTreeMap<String, Vec<String>>,
    /// workspace 成员相对目录(非空 = 本清单为 workspace 根,RXS-0091 单根锁)。
    pub workspace_members: Vec<String>,
}

impl Manifest {
    /// 解析 rurix.toml 文本(RXS-0089/0090)。
    pub fn parse(text: &str) -> PkgResult<Manifest> {
        let root = toml::parse(text).map_err(PkgError::ManifestInvalid)?;

        let pkg = root
            .get("package")
            .and_then(Value::as_table)
            .ok_or_else(|| PkgError::ManifestInvalid("缺 [package] 表".to_owned()))?;

        let name = pkg
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| PkgError::ManifestInvalid("缺 package.name".to_owned()))?
            .to_owned();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(PkgError::ManifestInvalid(format!(
                "非法 package.name {name:?}"
            )));
        }

        let version_str = pkg
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| PkgError::ManifestInvalid("缺 package.version".to_owned()))?;
        let version = Version::parse(version_str).map_err(PkgError::ManifestInvalid)?;

        // 无 build.rs 红线(09 §7.1):build 缺省即 declarative,唯一合法值 "declarative"。
        if let Some(build) = pkg.get("build") {
            match build.as_str() {
                Some("declarative") => {}
                _ => {
                    return Err(PkgError::ManifestInvalid(
                        "package.build 唯一合法值为 \"declarative\"(无 build.rs 红线,09 §7.1;硬需求登记 RD-###)".to_owned(),
                    ));
                }
            }
        }

        // edition 声明(RXS-0177~0179,RFC-0008):缺省 = 首个 edition(向后兼容);
        // 值非字符串 → RX7005(类型错误,复用 ManifestInvalid);未知值 → RX7020
        // (strict-only,无 fallback,不回退缺省,P-01)。
        let edition = match pkg.get("edition") {
            None => Edition::FIRST,
            Some(v) => {
                let s = v.as_str().ok_or_else(|| {
                    PkgError::ManifestInvalid("package.edition 须为字符串".to_owned())
                })?;
                Edition::parse(s).map_err(|e| match e {
                    EditionError::Unknown(bad) => PkgError::EditionUnknown(format!(
                        "package.edition {bad:?} 不在合法集合 {VALID_EDITIONS:?}(strict-only,无 fallback;新增 edition 经 Full RFC)"
                    )),
                })?
            }
        };

        let mut dependencies = BTreeMap::new();
        if let Some(deps) = root.get("dependencies").and_then(Value::as_table) {
            for (dep_name, spec) in deps {
                let dep = parse_dependency(dep_name, spec)?;
                dependencies.insert(dep_name.clone(), dep);
            }
        }

        let mut features = BTreeMap::new();
        if let Some(feats) = root.get("features").and_then(Value::as_table) {
            for (feat, list) in feats {
                let items = list
                    .as_str_array()
                    .map_err(|e| PkgError::ManifestInvalid(format!("feature {feat:?}: {e}")))?;
                features.insert(feat.clone(), items);
            }
        }

        let mut workspace_members = Vec::new();
        if let Some(ws) = root.get("workspace").and_then(Value::as_table)
            && let Some(members) = ws.get("members")
        {
            workspace_members = members
                .as_str_array()
                .map_err(|e| PkgError::ManifestInvalid(format!("workspace.members: {e}")))?;
        }

        Ok(Manifest {
            name,
            version,
            edition,
            dependencies,
            features,
            workspace_members,
        })
    }

    pub fn is_workspace_root(&self) -> bool {
        !self.workspace_members.is_empty()
    }
}

fn parse_dependency(name: &str, spec: &Value) -> PkgResult<Dependency> {
    let tbl = spec
        .as_table()
        .ok_or_else(|| PkgError::ManifestInvalid(format!("依赖 {name:?} 须为内联表 {{ ... }}")))?;
    let has_path = tbl.contains_key("path");
    let has_git = tbl.contains_key("git");
    let has_archive = tbl.contains_key("archive");
    let source_count = [has_path, has_git, has_archive]
        .iter()
        .filter(|b| **b)
        .count();
    if source_count != 1 {
        return Err(PkgError::ManifestInvalid(format!(
            "依赖 {name:?} 须恰好一个来源键(path/git/archive),实得 {source_count}"
        )));
    }
    let get_str = |k: &str| -> PkgResult<String> {
        tbl.get(k)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| PkgError::ManifestInvalid(format!("依赖 {name:?} 的 {k} 须为字符串")))
    };
    let source = if has_path {
        Source::Path(get_str("path")?)
    } else if has_git {
        let url = get_str("git")?;
        let rev = tbl.get("rev").and_then(Value::as_str).ok_or_else(|| {
            PkgError::ManifestInvalid(format!("git 依赖 {name:?} 须携带 rev(精确提交)"))
        })?;
        Source::Git {
            url,
            rev: rev.to_owned(),
        }
    } else {
        let url = get_str("archive")?;
        let sha = tbl.get("sha256").and_then(Value::as_str).ok_or_else(|| {
            PkgError::ManifestInvalid(format!("archive 依赖 {name:?} 须携带 sha256"))
        })?;
        if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PkgError::ManifestInvalid(format!(
                "archive 依赖 {name:?} 的 sha256 须为 64 位十六进制"
            )));
        }
        Source::Archive {
            url,
            sha256: sha.to_owned(),
        }
    };

    let features = match tbl.get("features") {
        Some(v) => v
            .as_str_array()
            .map_err(|e| PkgError::ManifestInvalid(format!("依赖 {name:?} features: {e}")))?,
        None => Vec::new(),
    };
    let default_features = match tbl.get("default-features") {
        Some(Value::Boolean(b)) => *b,
        Some(_) => {
            return Err(PkgError::ManifestInvalid(format!(
                "依赖 {name:?} 的 default-features 须为布尔"
            )));
        }
        None => true,
    };

    Ok(Dependency {
        source,
        features,
        default_features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0089
    #[test]
    fn parses_minimal_manifest() {
        let m = Manifest::parse("[package]\nname = \"app\"\nversion = \"1.2.3\"\n").unwrap();
        assert_eq!(m.name, "app");
        assert_eq!(m.version.to_string(), "1.2.3");
        assert!(m.dependencies.is_empty());
        assert!(!m.is_workspace_root());
    }

    //@ spec: RXS-0089
    #[test]
    fn rejects_non_declarative_build_no_build_rs() {
        // 无 build.rs 红线(09 §7.1):build 仅许 "declarative"。
        let err =
            Manifest::parse("[package]\nname = \"a\"\nversion = \"0.1.0\"\nbuild = \"build.rs\"\n")
                .unwrap_err();
        assert_eq!(err.code(), "RX7005");
        // 显式声明 declarative 合法。
        assert!(
            Manifest::parse(
                "[package]\nname = \"a\"\nversion = \"0.1.0\"\nbuild = \"declarative\"\n"
            )
            .is_ok()
        );
    }

    //@ spec: RXS-0089
    #[test]
    fn rejects_missing_fields_and_bad_version() {
        assert_eq!(
            Manifest::parse("[package]\nversion = \"0.1.0\"\n")
                .unwrap_err()
                .code(),
            "RX7005"
        );
        assert_eq!(
            Manifest::parse("[package]\nname = \"a\"\nversion = \"1.0\"\n")
                .unwrap_err()
                .code(),
            "RX7005"
        );
    }

    //@ spec: RXS-0090
    #[test]
    fn parses_three_sources_and_locators() {
        let text = r#"
[package]
name = "app"
version = "0.1.0"

[dependencies]
p = { path = "../p" }
g = { git = "https://h/r", rev = "deadbeef" }
a = { archive = "https://h/a.tar", sha256 = "0000000000000000000000000000000000000000000000000000000000000000" }
"#;
        let m = Manifest::parse(text).unwrap();
        assert_eq!(m.dependencies["p"].source.locator(), "path:../p");
        assert_eq!(
            m.dependencies["g"].source.locator(),
            "git:https://h/r#deadbeef"
        );
        assert!(m.dependencies["p"].source.is_local());
        assert!(!m.dependencies["g"].source.is_local());
    }

    //@ spec: RXS-0177
    #[test]
    fn edition_default_and_explicit_and_type_error() {
        // RXS-0177:缺 edition 键 → 首个 edition(向后兼容,既有清单 0-byte)。
        let m = Manifest::parse("[package]\nname = \"app\"\nversion = \"0.1.0\"\n").unwrap();
        assert_eq!(m.edition, Edition::FIRST);
        assert_eq!(m.edition, Edition::Edition2026);
        // 显式 edition = "2026" 解析为 Edition2026。
        let m2 =
            Manifest::parse("[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n")
                .unwrap();
        assert_eq!(m2.edition, Edition::Edition2026);
        // edition 值非字符串(整数)→ RX7005(类型错误复用 ManifestInvalid,不新增码)。
        let err =
            Manifest::parse("[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = 2026\n")
                .unwrap_err();
        assert_eq!(err.code(), "RX7005");
    }

    //@ spec: RXS-0178
    #[test]
    fn edition_parse_is_deterministic_and_validates() {
        // RXS-0178:Edition::parse 确定性纯函数,合法集 { "2026" }。
        assert_eq!(Edition::parse("2026"), Ok(Edition::Edition2026));
        // 两次解析相同输入结果一致(确定性)。
        assert_eq!(Edition::parse("2026"), Edition::parse("2026"));
        // 合法集合外 → Err::Unknown(strict-only)。
        assert_eq!(
            Edition::parse("2099"),
            Err(EditionError::Unknown("2099".to_owned()))
        );
        assert!(Edition::parse("").is_err());
        assert!(Edition::parse("latest").is_err());
        // 合法集合常量与 parse 行为一致(RFC-0008 §4.2 一字对齐)。
        assert_eq!(VALID_EDITIONS, &["2026"]);
        for e in VALID_EDITIONS {
            assert!(Edition::parse(e).is_ok());
        }
    }

    //@ spec: RXS-0179
    #[test]
    fn rejects_unknown_edition_rx7020_no_fallback() {
        // RXS-0179:未知 edition → RX7020 strict-only 拒,不回退缺省。
        let err =
            Manifest::parse("[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2099\"\n")
                .unwrap_err();
        assert_eq!(err.code(), "RX7020");
        assert!(matches!(err, PkgError::EditionUnknown(_)));
        // 不回退缺省:未知 edition 是 Err,不是 Ok(Edition::FIRST)。
        assert!(
            Manifest::parse("[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2015\"\n")
                .is_err()
        );
    }

    //@ spec: RXS-0180
    #[test]
    fn edition_is_stable_surface_anchor() {
        // RXS-0180:edition 作 stable 面版本锚边界;规范字符串进 stable 快照内容。
        assert_eq!(Edition::FIRST.as_str(), "2026");
        assert_eq!(Edition::Edition2026.as_str(), "2026");
        // 首期 edition-gated 行为差异 = 空集(机制锚点,RFC-0008 §9 Q-Scope)。
        assert!(!Edition::Edition2026.behavior_differs(Edition::Edition2026));
        // edition 合法值集作 stable 快照基准的存在性(stable 面 = 含 edition 值集)。
        assert!(VALID_EDITIONS.contains(&Edition::FIRST.as_str()));
    }

    //@ spec: RXS-0090
    #[test]
    fn rejects_multiple_or_missing_source_and_missing_pin() {
        // 多来源键
        assert_eq!(
            Manifest::parse(
                "[package]\nname=\"a\"\nversion=\"0.1.0\"\n[dependencies]\nx = { path = \"p\", git = \"u\" }\n"
            )
            .unwrap_err()
            .code(),
            "RX7005"
        );
        // git 缺 rev
        assert_eq!(
            Manifest::parse(
                "[package]\nname=\"a\"\nversion=\"0.1.0\"\n[dependencies]\nx = { git = \"u\" }\n"
            )
            .unwrap_err()
            .code(),
            "RX7005"
        );
    }
}
