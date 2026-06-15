//! 依赖解析图与 feature additive-v1 合一(RXS-0091)。
//!
//! workspace 单根锁(09 §7.2):整个 workspace 共享单一解析图与单一 rurix.lock。
//! feature 统一为 additive-v1(`unification="selected"`):同一依赖被多上游以不同
//! feature 启用时,最终 feature 集 = 各上游所选并集(加性单调,不动点收敛)。
//! 冲突(来源/pin 不相容 / feature 引用不存在)→ [`PkgError::ResolutionConflict`]
//! (RX7006)。来源 I/O 与内容哈希经 [`PackageLoader`] 注入(纯逻辑可单测)。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::error::{PkgError, PkgResult};
use crate::manifest::{Manifest, Source, Version};

/// 已加载的依赖包(loader 负责 path/vendor 定位、offline 裁决与内容哈希)。
pub struct LoadedPackage {
    pub manifest: Manifest,
    pub content_sha256: String,
    /// loader 归一后的来源(通常与请求一致)。
    pub source: Source,
}

/// 来源 → 包加载抽象(filesystem 实现见 [`crate::vendor`])。
pub trait PackageLoader {
    fn load(&self, name: &str, source: &Source) -> PkgResult<LoadedPackage>;
}

/// 解析图中的一个包节点(确定性:feature/deps 排序)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: Version,
    pub source: Source,
    pub content_sha256: String,
    pub features: Vec<String>,
    pub deps: Vec<String>,
}

/// 解析图(单根锁,RXS-0091)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveGraph {
    pub root: String,
    pub nodes: BTreeMap<String, ResolvedPackage>,
}

struct NodeState {
    manifest: Manifest,
    source: Source,
    content_sha256: String,
    requested: BTreeSet<String>,
}

/// 构建解析图(RXS-0091)。`root_content_sha256` 为根包内容树哈希(RXS-0093)。
pub fn resolve(
    root: &Manifest,
    root_content_sha256: &str,
    loader: &dyn PackageLoader,
) -> PkgResult<ResolveGraph> {
    let mut nodes: BTreeMap<String, NodeState> = BTreeMap::new();
    nodes.insert(
        root.name.clone(),
        NodeState {
            manifest: root.clone(),
            source: Source::Path(".".to_owned()),
            content_sha256: root_content_sha256.to_owned(),
            requested: default_requested(root),
        },
    );

    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root.name.clone());

    while let Some(name) = queue.pop_front() {
        // 取出当前 requested 与 manifest(克隆以释放对 nodes 的借用)。
        let (manifest, requested) = {
            let st = &nodes[&name];
            (st.manifest.clone(), st.requested.clone())
        };
        let (_selected, dep_feats) = expand_features(&manifest, &requested)?;

        for (dep_name, dep) in &manifest.dependencies {
            // 该依赖此上游请求的 feature:dep.features ∪ default(若启用) ∪ "<dep>/<feat>"。
            let mut dep_req: BTreeSet<String> = dep.features.iter().cloned().collect();
            if let Some(extra) = dep_feats.get(dep_name) {
                dep_req.extend(extra.iter().cloned());
            }

            match nodes.get_mut(dep_name) {
                Some(existing) => {
                    // 单根锁冲突检测:同名依赖来源/pin 不相容(RXS-0091)。
                    if existing.source.locator() != dep.source.locator() {
                        return Err(PkgError::ResolutionConflict(format!(
                            "依赖 {dep_name:?} 来源不相容:{} vs {}",
                            existing.source.locator(),
                            dep.source.locator()
                        )));
                    }
                    let grew = add_default_marker(&mut existing.requested, &existing.manifest, dep)
                        | extend_grew(&mut existing.requested, &dep_req);
                    if grew {
                        queue.push_back(dep_name.clone());
                    }
                }
                None => {
                    let loaded = loader.load(dep_name, &dep.source)?;
                    if &loaded.manifest.name != dep_name {
                        return Err(PkgError::ResolutionConflict(format!(
                            "依赖 {dep_name:?} 的清单 package.name 为 {:?}(不匹配)",
                            loaded.manifest.name
                        )));
                    }
                    let mut requested = dep_req.clone();
                    add_default_marker(&mut requested, &loaded.manifest, dep);
                    nodes.insert(
                        dep_name.clone(),
                        NodeState {
                            manifest: loaded.manifest,
                            source: loaded.source,
                            content_sha256: loaded.content_sha256,
                            requested,
                        },
                    );
                    queue.push_back(dep_name.clone());
                }
            }
        }
    }

    // 定稿:逐节点展开最终 feature 集 + 直接依赖名(确定性排序)。
    let mut out: BTreeMap<String, ResolvedPackage> = BTreeMap::new();
    for (name, st) in &nodes {
        let (selected, _) = expand_features(&st.manifest, &st.requested)?;
        let deps: Vec<String> = st.manifest.dependencies.keys().cloned().collect();
        out.insert(
            name.clone(),
            ResolvedPackage {
                name: name.clone(),
                version: st.manifest.version.clone(),
                source: st.source.clone(),
                content_sha256: st.content_sha256.clone(),
                features: selected.into_iter().collect(),
                deps,
            },
        );
    }
    Ok(ResolveGraph {
        root: root.name.clone(),
        nodes: out,
    })
}

/// 根包默认启用 feature:有 `default` 则启用之(cargo-like 默认开)。
fn default_requested(m: &Manifest) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    if m.features.contains_key("default") {
        s.insert("default".to_owned());
    }
    s
}

/// 依赖启用 default-features 且其清单有 `default` feature 时,补 "default" 标记。
/// 返回是否使集合增长。
fn add_default_marker(
    req: &mut BTreeSet<String>,
    dep_manifest: &Manifest,
    dep: &crate::manifest::Dependency,
) -> bool {
    if dep.default_features && dep_manifest.features.contains_key("default") {
        req.insert("default".to_owned())
    } else {
        false
    }
}

fn extend_grew(target: &mut BTreeSet<String>, src: &BTreeSet<String>) -> bool {
    let before = target.len();
    target.extend(src.iter().cloned());
    target.len() != before
}

/// feature 展开结果:(该包启用的 feature 集, 各依赖额外请求的 feature)。
type Expanded = (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>);

/// feature 展开(加性闭包):requested → 启用 feature 集 + 各依赖额外 feature。
fn expand_features(m: &Manifest, requested: &BTreeSet<String>) -> PkgResult<Expanded> {
    let mut selected: BTreeSet<String> = BTreeSet::new();
    let mut dep_feats: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut queue: VecDeque<String> = requested.iter().cloned().collect();

    while let Some(tok) = queue.pop_front() {
        if let Some((dep, feat)) = tok.split_once('/') {
            if !m.dependencies.contains_key(dep) {
                return Err(PkgError::ResolutionConflict(format!(
                    "feature 引用不存在的依赖 {dep:?}(于 {:?})",
                    tok
                )));
            }
            dep_feats
                .entry(dep.to_owned())
                .or_default()
                .insert(feat.to_owned());
            continue;
        }
        if m.features.contains_key(&tok) {
            if selected.insert(tok.clone()) {
                for act in &m.features[&tok] {
                    queue.push_back(act.clone());
                }
            }
            continue;
        }
        // 裸 token 等于依赖名:启用该依赖(MVP 依赖恒在图中,no-op)。
        if m.dependencies.contains_key(&tok) {
            continue;
        }
        return Err(PkgError::ResolutionConflict(format!(
            "feature {tok:?} 在包 {:?} 中既非 feature 也非依赖",
            m.name
        )));
    }
    Ok((selected, dep_feats))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLoader {
        pkgs: BTreeMap<String, (Manifest, String)>,
    }
    impl PackageLoader for FakeLoader {
        fn load(&self, name: &str, source: &Source) -> PkgResult<LoadedPackage> {
            let (m, sha) = self
                .pkgs
                .get(name)
                .ok_or_else(|| PkgError::SourceUnreachable(format!("无 {name:?}")))?;
            Ok(LoadedPackage {
                manifest: m.clone(),
                content_sha256: sha.clone(),
                source: source.clone(),
            })
        }
    }

    fn m(text: &str) -> Manifest {
        Manifest::parse(text).unwrap()
    }

    //@ spec: RXS-0091
    #[test]
    fn builds_graph_over_path_deps() {
        let root = m(
            "[package]\nname=\"app\"\nversion=\"0.1.0\"\n[dependencies]\nfoo = { path = \"../foo\" }\n",
        );
        let foo = m(
            "[package]\nname=\"foo\"\nversion=\"0.2.0\"\n[dependencies]\nbar = { path = \"../bar\" }\n",
        );
        let bar = m("[package]\nname=\"bar\"\nversion=\"0.3.0\"\n");
        let loader = FakeLoader {
            pkgs: BTreeMap::from([
                ("foo".to_owned(), (foo, "f".repeat(64))),
                ("bar".to_owned(), (bar, "b".repeat(64))),
            ]),
        };
        let g = resolve(&root, &"a".repeat(64), &loader).unwrap();
        assert_eq!(g.root, "app");
        assert_eq!(
            g.nodes.keys().cloned().collect::<Vec<_>>(),
            vec!["app", "bar", "foo"]
        );
        assert_eq!(g.nodes["app"].deps, vec!["foo"]);
        assert_eq!(g.nodes["foo"].deps, vec!["bar"]);
        assert_eq!(g.nodes["bar"].version.to_string(), "0.3.0");
    }

    //@ spec: RXS-0091
    #[test]
    fn feature_union_is_additive() {
        // app 经 default 启用 dep foo 的 fa;另一上游 mid 启用 foo 的 fb;
        // 最终 foo.features = {fa, fb} 并集(加性合一)。
        let root = m(concat!(
            "[package]\nname=\"app\"\nversion=\"0.1.0\"\n",
            "[dependencies]\n",
            "foo = { path = \"f\", default-features = false, features = [\"fa\"] }\n",
            "mid = { path = \"m\" }\n",
        ));
        let mid = m(concat!(
            "[package]\nname=\"mid\"\nversion=\"0.1.0\"\n",
            "[dependencies]\nfoo = { path = \"f\", default-features = false, features = [\"fb\"] }\n",
        ));
        let foo = m(concat!(
            "[package]\nname=\"foo\"\nversion=\"0.1.0\"\n",
            "[features]\nfa = []\nfb = []\n",
        ));
        let loader = FakeLoader {
            pkgs: BTreeMap::from([
                ("mid".to_owned(), (mid, "m".repeat(64))),
                ("foo".to_owned(), (foo, "f".repeat(64))),
            ]),
        };
        let g = resolve(&root, &"a".repeat(64), &loader).unwrap();
        assert_eq!(
            g.nodes["foo"].features,
            vec!["fa".to_owned(), "fb".to_owned()]
        );
    }

    //@ spec: RXS-0091
    #[test]
    fn source_conflict_is_rejected() {
        let root = m(concat!(
            "[package]\nname=\"app\"\nversion=\"0.1.0\"\n",
            "[dependencies]\n",
            "foo = { path = \"f1\" }\n",
            "mid = { path = \"m\" }\n",
        ));
        let mid = m(concat!(
            "[package]\nname=\"mid\"\nversion=\"0.1.0\"\n",
            "[dependencies]\nfoo = { path = \"f2\" }\n",
        ));
        let foo = m("[package]\nname=\"foo\"\nversion=\"0.1.0\"\n");
        let loader = FakeLoader {
            pkgs: BTreeMap::from([
                ("mid".to_owned(), (mid, "m".repeat(64))),
                ("foo".to_owned(), (foo, "f".repeat(64))),
            ]),
        };
        let err = resolve(&root, &"a".repeat(64), &loader).unwrap_err();
        assert_eq!(err.code(), "RX7006");
    }

    //@ spec: RXS-0091
    #[test]
    fn nonexistent_feature_is_rejected() {
        let root = m(concat!(
            "[package]\nname=\"app\"\nversion=\"0.1.0\"\n",
            "[features]\ndefault = [\"nope\"]\n",
        ));
        let loader = FakeLoader {
            pkgs: BTreeMap::new(),
        };
        let err = resolve(&root, &"a".repeat(64), &loader).unwrap_err();
        assert_eq!(err.code(), "RX7006");
    }
}
