//! 发布 bundle 清单与语言本体 / NVIDIA 再分发组件分离打包(spec/release.md RXS-0136)。
//!
//! 发布 bundle **分区**为「语言本体」([`Partition::LanguageCore`],Rurix 自研编译器 /
//! 运行时 / 标准库,自有许可)与「NVIDIA 再分发组件」([`Partition::NvidiaRedist`],
//! 仅 Attachment A 白名单最小集——MVP 实际只需 `libdevice.10.bc`,cuBLAS 绑定包按需
//! 附带 `cublas64_*.dll`)。**完整 Toolkit / 驱动 / Nsight 永不捆绑**(许可红线 r6):
//! NVIDIA 分区中任一非 Attachment A 白名单组件即审计违例([`audit_redistribution`]),
//! 延续 M5.4 `ci/check_redistribution.py` 口径。

use std::fmt;

/// 发布 bundle 分区(语言本体 ⟂ NVIDIA 再分发组件,RXS-0136)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Partition {
    /// 语言本体:Rurix 自研编译器 / 运行时 / 标准库(自有许可)。
    LanguageCore,
    /// NVIDIA 再分发组件:仅 Attachment A 白名单最小集(完整 Toolkit/驱动/Nsight 永不捆绑)。
    NvidiaRedist,
}

impl Partition {
    /// 稳定字符串标签(SBOM / 清单序列化用)。
    pub fn label(self) -> &'static str {
        match self {
            Partition::LanguageCore => "language-core",
            Partition::NvidiaRedist => "nvidia-redist",
        }
    }
}

impl fmt::Display for Partition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 单个分发组件(干名 + 版本 + 许可标识 + 分区 + content SHA-256 十六进制)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// 产物干名(如 `rurixc.exe` / `libdevice.10.bc` / `cublas64_12.dll`)。
    pub name: String,
    /// 版本号(语言本体组件随同一版号原子分发,RXS-0135)。
    pub version: String,
    /// 许可标识(SPDX license id 或再分发条款标签)。
    pub license: String,
    /// 所属分区。
    pub partition: Partition,
    /// 组件内容 SHA-256 十六进制(64 字符)。
    pub sha256: String,
}

/// 发布 bundle 清单:同一版号下的全部分发组件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifest {
    /// 发布版号(语言本体组件须同号,RXS-0135)。
    pub rurix_version: String,
    /// 全部分发组件(序列化前按 `name` 排序,确定性)。
    pub components: Vec<Component>,
}

/// Attachment A 白名单审计判定(RXS-0136 / r6;延续 M5.4 check_redistribution)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedistributionAudit {
    /// 综合判定:NVIDIA 分区组件是否全部落 Attachment A 白名单最小集。
    pub pass: bool,
    /// 白名单外的 NVIDIA 组件干名(违例项;`pass=false` 时非空)。
    pub violations: Vec<String>,
}

/// 发布资产 3 组件完备最小集(RXS-0218 / 裁决 D):新版本发布清单须含编译器
/// (`rx.exe`)、引导器(`rurixup.exe`)与 **crt-static** 运行时静态库
/// (`rurix_rt_cabi.lib`)——v1.0.0 资产缺 `rurix_rt_cabi.lib`,无 Rust 环境含 GPU
/// 面的 `rx build` 必死,EA1.2 本期修口径。全 [`Partition::LanguageCore`]。
pub const RELEASE_COMPONENTS: [&str; 3] = ["rurix_rt_cabi.lib", "rurixup.exe", "rx.exe"];

/// 发布资产 3 组件完备判定(RXS-0218):`complete` = [`RELEASE_COMPONENTS`] 全部
/// 出现于 bundle 组件干名集;`missing` 为缺失的必需组件干名(字典序,`complete=false`
/// 时非空)。**发布侧新版本查询**,非 `run_release` hard-block 子门(既有 8 子门 0-byte)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseCompleteness {
    /// 3 组件完备最小集是否齐备。
    pub complete: bool,
    /// 缺失的必需组件干名(字典序枚举;`complete=false` 时非空)。
    pub missing: Vec<String>,
}

impl BundleManifest {
    /// 构造空清单(指定版号)。
    pub fn new(rurix_version: impl Into<String>) -> Self {
        BundleManifest {
            rurix_version: rurix_version.into(),
            components: Vec::new(),
        }
    }

    /// 追加组件。
    pub fn push(&mut self, component: Component) {
        self.components.push(component);
    }

    /// 按分区筛选组件(只读引用,按 `name` 字典序稳定排序)。
    pub fn partition(&self, p: Partition) -> Vec<&Component> {
        let mut out: Vec<&Component> = self
            .components
            .iter()
            .filter(|c| c.partition == p)
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// 语言本体组件须**同一版号**原子分发(RXS-0135):任一语言本体组件版本
    /// 与 bundle 版号不符即返回 `false`(NVIDIA 再分发组件各有上游版本,豁免)。
    pub fn language_core_versions_uniform(&self) -> bool {
        self.partition(Partition::LanguageCore)
            .iter()
            .all(|c| c.version == self.rurix_version)
    }

    /// 发布资产 3 组件完备判定(RXS-0218 / 裁决 D):bundle 组件干名集须覆盖
    /// [`RELEASE_COMPONENTS`]("rx.exe" + "rurixup.exe" + "rurix_rt_cabi.lib")全集,
    /// 缺任一必需组件 → `complete=false` + `missing` 枚举(字典序)。**新版本发布查询**
    /// (非 hard-block 子门;老版本两件清单 `complete=false` 是如实,不阻断既有语义)。
    pub fn release_completeness(&self) -> ReleaseCompleteness {
        let present: std::collections::BTreeSet<&str> =
            self.components.iter().map(|c| c.name.as_str()).collect();
        let mut missing: Vec<String> = RELEASE_COMPONENTS
            .iter()
            .filter(|req| !present.contains(**req))
            .map(|req| req.to_string())
            .collect();
        missing.sort();
        ReleaseCompleteness {
            complete: missing.is_empty(),
            missing,
        }
    }

    /// SHA256SUMS 清单确定性序列化(RXS-0218):组件按**干名字典序**,每行标准
    /// `sha256sum` 双空格格式 `<sha256>␣␣<name>`(每资产字节 == bundle.json 组件
    /// `sha256` 的对象,一比一内容寻址,无第二 digest 域)。同一 bundle 两次生成
    /// **逐字节一致**(纯函数确定性,复用 to_json 的干名字典序纪律)。
    pub fn sha256sums(&self) -> String {
        let mut comps = self.components.clone();
        comps.sort_by(|a, b| a.name.cmp(&b.name));
        let mut s = String::new();
        for c in &comps {
            s.push_str(&c.sha256);
            s.push_str("  ");
            s.push_str(&c.name);
            s.push('\n');
        }
        s
    }

    /// bundle 清单确定性 JSON 序列化(组件按干名字典序;自 `main.rs` 上移,
    /// 序列化字节 0-byte 不变):`main.rs` 写出 `bundle.json` 与 channel 清单的
    /// `bundle_manifest_sha256` 内容寻址引用(RXS-0185)共用同一字节流。
    pub fn to_json(&self) -> String {
        let mut comps = self.components.clone();
        comps.sort_by(|a, b| a.name.cmp(&b.name));
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!(
            "  \"rurix_version\": \"{}\",\n",
            crate::json_escape(&self.rurix_version)
        ));
        s.push_str("  \"components\": [\n");
        for (i, c) in comps.iter().enumerate() {
            let comma = if i + 1 < comps.len() { "," } else { "" };
            s.push_str("    {\n");
            s.push_str(&format!(
                "      \"name\": \"{}\",\n",
                crate::json_escape(&c.name)
            ));
            s.push_str(&format!(
                "      \"version\": \"{}\",\n",
                crate::json_escape(&c.version)
            ));
            s.push_str(&format!(
                "      \"license\": \"{}\",\n",
                crate::json_escape(&c.license)
            ));
            s.push_str(&format!(
                "      \"partition\": \"{}\",\n",
                c.partition.label()
            ));
            s.push_str(&format!(
                "      \"sha256\": \"{}\"\n",
                crate::json_escape(&c.sha256)
            ));
            s.push_str(&format!("    }}{comma}\n"));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// 从 `bundle.json` 解析(MR-0009:工具链前端 install 消费已发布 bundle;
    /// 确定性 round-trip `from_json(to_json(b)) == b`,组件字典序)。手写极简解析
    /// (零外部依赖,仅识别本 crate `to_json` 产出的规范形态)。
    pub fn from_json(text: &str) -> Result<BundleManifest, String> {
        let field = |line: &str, key: &str| -> Option<String> {
            line.trim()
                .strip_prefix(&format!("\"{key}\":"))
                .map(|r| r.trim().trim_end_matches(',').trim_matches('"').to_string())
        };
        let mut rurix_version: Option<String> = None;
        let mut components: Vec<Component> = Vec::new();
        let mut name = None;
        let mut version = None;
        let mut license = None;
        let mut partition = None;
        for line in text.lines() {
            if rurix_version.is_none()
                && let Some(v) = field(line, "rurix_version")
            {
                rurix_version = Some(v);
                continue;
            }
            if let Some(v) = field(line, "name") {
                name = Some(v);
            } else if let Some(v) = field(line, "version") {
                version = Some(v);
            } else if let Some(v) = field(line, "license") {
                license = Some(v);
            } else if let Some(v) = field(line, "partition") {
                partition = Some(v);
            } else if let Some(v) = field(line, "sha256") {
                let (n, ver, lic, part) = (
                    name.take().ok_or("组件缺 name")?,
                    version.take().ok_or("组件缺 version")?,
                    license.take().ok_or("组件缺 license")?,
                    partition.take().ok_or("组件缺 partition")?,
                );
                let partition = match part.as_str() {
                    "language-core" => Partition::LanguageCore,
                    "nvidia-redist" => Partition::NvidiaRedist,
                    other => return Err(format!("未知分区 `{other}`")),
                };
                components.push(Component {
                    name: n,
                    version: ver,
                    license: lic,
                    partition,
                    sha256: v,
                });
            }
        }
        let rurix_version = rurix_version.ok_or("bundle.json 缺 rurix_version")?;
        let mut bundle = BundleManifest {
            rurix_version,
            components,
        };
        bundle.components.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(bundle)
    }
}

/// 判定组件干名是否为 NVIDIA libdevice bitcode(`libdevice.<digits>.bc`)。
fn is_libdevice(name: &str) -> bool {
    match name
        .strip_prefix("libdevice.")
        .and_then(|s| s.strip_suffix(".bc"))
    {
        Some(mid) => !mid.is_empty() && mid.bytes().all(|b| b.is_ascii_digit()),
        None => false,
    }
}

/// 判定组件干名是否为 cuBLAS runtime DLL(`cublas64_<digits>.dll` /
/// `cublasLt64_<digits>.dll`;对齐 M5.4 check_redistribution 断言 3c 白名单正则)。
fn is_cublas_runtime(name: &str) -> bool {
    for stem in ["cublas64_", "cublasLt64_"] {
        if let Some(mid) = name.strip_prefix(stem).and_then(|s| s.strip_suffix(".dll"))
            && !mid.is_empty()
            && mid.bytes().all(|b| b.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

/// Attachment A 白名单最小集判定(RXS-0136):仅 libdevice bitcode 与 cuBLAS
/// runtime DLL;完整 Toolkit(`nvcc`/`ptxas`)/ 驱动 / Nsight 等一概不在白名单。
pub fn is_attachment_a_whitelisted(name: &str) -> bool {
    is_libdevice(name) || is_cublas_runtime(name)
}

/// NVIDIA 再分发白名单审计(RXS-0136 / r6):NVIDIA 分区中任一非 Attachment A
/// 白名单组件即违例;语言本体分区不参与本审计。
pub fn audit_redistribution(bundle: &BundleManifest) -> RedistributionAudit {
    let mut violations: Vec<String> = bundle
        .partition(Partition::NvidiaRedist)
        .iter()
        .filter(|c| !is_attachment_a_whitelisted(&c.name))
        .map(|c| c.name.clone())
        .collect();
    violations.sort();
    RedistributionAudit {
        pass: violations.is_empty(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(name: &str, ver: &str, p: Partition) -> Component {
        Component {
            name: name.to_string(),
            version: ver.to_string(),
            license: "Apache-2.0".to_string(),
            partition: p,
            sha256: "00".repeat(32),
        }
    }

    //@ spec: RXS-0136
    // 语言本体 ⟂ NVIDIA 再分发组件分区:NVIDIA 分区仅容 Attachment A 白名单最小集
    // (libdevice bitcode + cuBLAS runtime DLL),完整 Toolkit/驱动/Nsight 即违例(r6)。
    #[test]
    fn bundle_separates_core_from_nvidia_redist() {
        let mut b = BundleManifest::new("0.1.0");
        b.push(comp("rurixc.exe", "0.1.0", Partition::LanguageCore));
        b.push(comp("rurix_rt.dll", "0.1.0", Partition::LanguageCore));
        b.push(comp("libdevice.10.bc", "12.3", Partition::NvidiaRedist));
        b.push(comp("cublas64_12.dll", "12.3", Partition::NvidiaRedist));

        // 分区筛选确定(字典序)。
        let core = b.partition(Partition::LanguageCore);
        assert_eq!(core.len(), 2);
        assert_eq!(core[0].name, "rurix_rt.dll");
        let redist = b.partition(Partition::NvidiaRedist);
        assert_eq!(redist.len(), 2);

        // Attachment A 白名单识别。
        assert!(is_attachment_a_whitelisted("libdevice.10.bc"));
        assert!(is_attachment_a_whitelisted("cublas64_12.dll"));
        assert!(is_attachment_a_whitelisted("cublasLt64_12.dll"));
        // 完整 Toolkit / 驱动 / Nsight 永不入白名单(r6)。
        assert!(!is_attachment_a_whitelisted("nvcc.exe"));
        assert!(!is_attachment_a_whitelisted("ptxas.exe"));
        assert!(!is_attachment_a_whitelisted("nsight-compute.exe"));
        assert!(!is_attachment_a_whitelisted("nvcuda.dll"));
        assert!(!is_attachment_a_whitelisted("libdevice.bc")); // 缺版本号段
        assert!(!is_attachment_a_whitelisted("cublas64_.dll")); // 缺数字段

        // 全白名单 → 审计通过。
        let audit = audit_redistribution(&b);
        assert!(audit.pass);
        assert!(audit.violations.is_empty());

        // 语言本体同版号原子分发判据。
        assert!(b.language_core_versions_uniform());
    }

    //@ spec: RXS-0136
    // NVIDIA 分区混入白名单外组件(完整 Toolkit / Nsight)→ 审计违例,违例项可枚举。
    #[test]
    fn non_whitelisted_nvidia_component_fails_audit() {
        let mut b = BundleManifest::new("0.1.0");
        b.push(comp("libdevice.10.bc", "12.3", Partition::NvidiaRedist));
        b.push(comp(
            "nsight-compute.exe",
            "2024.1",
            Partition::NvidiaRedist,
        ));
        b.push(comp(
            "cudart64_full_toolkit.dll",
            "12.3",
            Partition::NvidiaRedist,
        ));

        let audit = audit_redistribution(&b);
        assert!(!audit.pass);
        assert_eq!(
            audit.violations,
            vec![
                "cudart64_full_toolkit.dll".to_string(),
                "nsight-compute.exe".to_string(),
            ]
        );
    }

    //@ spec: RXS-0135
    // 语言本体组件版号与 bundle 版号不符 → 非同一版号原子分发(RXS-0135 完整性判据之一)。
    #[test]
    fn language_core_version_skew_detected() {
        let mut b = BundleManifest::new("0.1.0");
        b.push(comp("rurixc.exe", "0.1.0", Partition::LanguageCore));
        b.push(comp("rx.exe", "0.0.9", Partition::LanguageCore)); // 版号偏移
        assert!(!b.language_core_versions_uniform());
    }

    //@ spec: RXS-0135
    // bundle.json 序列化/解析 round-trip 保真(MR-0009 工具链前端 install 消费依赖)。
    #[test]
    fn bundle_json_roundtrip() {
        let mut b = BundleManifest::new("1.0.0");
        b.push(comp("rurixup.exe", "1.0.0", Partition::LanguageCore));
        b.push(comp("libdevice.10.bc", "12.3", Partition::NvidiaRedist));
        let parsed = BundleManifest::from_json(&b.to_json()).expect("round-trip");
        let mut expected = b.clone();
        expected.components.sort_by(|a, c| a.name.cmp(&c.name));
        assert_eq!(parsed, expected);
        // 再序列化逐字节一致(确定性)。
        assert_eq!(parsed.to_json(), b.to_json());
    }

    //@ spec: RXS-0218
    // 发布资产 3 组件完备判定(裁决 D):rx.exe + rurixup.exe + rurix_rt_cabi.lib 全集
    // → complete;缺 crt-static .lib → complete=false + missing 枚举(缺件即红判据源);
    // 老版本两件清单如实 complete=false(不阻断既有 bundle 语义)。
    #[test]
    fn release_completeness_requires_three_components() {
        // 3 组件完备(新版本清单)。
        let mut full = BundleManifest::new("1.1.0");
        full.push(comp("rx.exe", "1.1.0", Partition::LanguageCore));
        full.push(comp("rurixup.exe", "1.1.0", Partition::LanguageCore));
        full.push(comp("rurix_rt_cabi.lib", "1.1.0", Partition::LanguageCore));
        let c = full.release_completeness();
        assert!(c.complete);
        assert!(c.missing.is_empty());

        // 缺 crt-static rurix_rt_cabi.lib(v1.0.0 两件老清单)→ 缺件红。
        let mut two = BundleManifest::new("1.0.0");
        two.push(comp("rx.exe", "1.0.0", Partition::LanguageCore));
        two.push(comp("rurixup.exe", "1.0.0", Partition::LanguageCore));
        let c2 = two.release_completeness();
        assert!(!c2.complete);
        assert_eq!(c2.missing, vec!["rurix_rt_cabi.lib".to_string()]);

        // 空 bundle → 三件全缺(字典序枚举)。
        let empty = BundleManifest::new("1.1.0");
        assert_eq!(
            empty.release_completeness().missing,
            vec![
                "rurix_rt_cabi.lib".to_string(),
                "rurixup.exe".to_string(),
                "rx.exe".to_string(),
            ]
        );
    }

    //@ spec: RXS-0218
    // SHA256SUMS 字典序确定性:组件按干名字典序、标准 sha256sum 双空格格式,
    // 同一 bundle 两次生成逐字节一致 + 每行 digest == 组件 sha256(一比一内容寻址)。
    #[test]
    fn sha256sums_lexicographic_deterministic() {
        let mut b = BundleManifest::new("1.1.0");
        // 刻意乱序 push,验证输出按干名字典序。
        let mut rx = comp("rx.exe", "1.1.0", Partition::LanguageCore);
        rx.sha256 = "11".repeat(32);
        let mut up = comp("rurixup.exe", "1.1.0", Partition::LanguageCore);
        up.sha256 = "22".repeat(32);
        let mut lib = comp("rurix_rt_cabi.lib", "1.1.0", Partition::LanguageCore);
        lib.sha256 = "33".repeat(32);
        b.push(rx);
        b.push(up);
        b.push(lib);

        let sums = b.sha256sums();
        // 干名字典序:rurix_rt_cabi.lib < rurixup.exe < rx.exe。
        let expected = format!(
            "{}  rurix_rt_cabi.lib\n{}  rurixup.exe\n{}  rx.exe\n",
            "33".repeat(32),
            "22".repeat(32),
            "11".repeat(32),
        );
        assert_eq!(sums, expected);
        // 两次生成逐字节一致(纯函数确定性)。
        assert_eq!(b.sha256sums(), sums);
        // 每行双空格 sha256sum 格式 + 每行 digest == 组件 sha256(一比一内容寻址)。
        for line in sums.lines() {
            let (digest, name) = line.split_once("  ").expect("双空格分隔");
            let matched = b
                .components
                .iter()
                .find(|c| c.name == name)
                .expect("行干名 == bundle 组件");
            assert_eq!(digest, matched.sha256);
        }
    }
}
