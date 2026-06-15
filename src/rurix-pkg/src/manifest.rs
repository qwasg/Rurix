//! rurix.toml 清单模型与解析(RXS-0089 清单格式 / RXS-0090 三来源)。
//!
//! 声明式无 build.rs(09 §7.1):`[package].build` 缺省或唯一合法值 `"declarative"`;
//! 任何其他值 → [`PkgError::ManifestInvalid`](crate::PkgError)(RX7005)。

use std::collections::BTreeMap;

use crate::error::{PkgError, PkgResult};
use crate::toml::{self, Value};

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
