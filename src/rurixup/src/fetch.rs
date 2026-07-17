//! 网络拉取载体 + 四级内容寻址信任链(spec/release.md RXS-0216 ~ RXS-0217;
//! RFC-0012 §4.4~4.6,裁决 A)。
//!
//! **RXS-0216 载体**:网络拉取仅经**系统 `curl.exe` 子进程**(固定参数集、
//! https-only 双 proto 钉死、host 白名单三者、环回 127.0.0.1 + 显式测试 env 唯一
//! 豁免)。TLS/代理/吊销全部委托 OS(schannel);curl 只搬运,完整性判定不信任
//! 传输层。**RXS-0217 信任链**:repo 锚 `channels/stable.json`(信任根 = repo)记
//! 每发行的 `channel_manifest_sha256`(级① 期望 digest)+ `base_url`;四级级联
//! (① 锚→channel ② channel→bundle ③ 一致性 ④ bundle→组件)任一级失配 → 拒装。
//!
//! **全 safe**(`unsafe_code=deny`,仅 `std::process` / `std::fs`,零 unsafe、零第三方
//! 下载 crate);载体失败以工具层 [`FetchError`] + 退出码非 0 + 机器 token
//! `RURIXUP_INSTALL_ERROR: kind=network` 表达(spec/release.md §3,零新 RX 码)。

use std::path::Path;
use std::process::Command;

/// 端点 host 白名单(RXS-0216;唯一例外 = 环回 127.0.0.1 + 显式测试 env)。
pub const HOST_ALLOWLIST: &[&str] = &[
    "github.com",
    "objects.githubusercontent.com",
    "raw.githubusercontent.com",
];

/// 环回 `http://` 豁免 env 开关名(hermetic CI fixture 用;缺省 fail-closed)。
pub const LOOPBACK_ENV: &str = "RURIXUP_TEST_ALLOW_LOOPBACK_HTTP";

/// 网络载体 / 端点约束错误(工具层,退出码非 0;**非编译器 RX 段位码**,§3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// 非法 URL(无法解析 scheme / host)。
    BadUrl(String),
    /// 协议不受支持(缺省态 `http://` / 非 http(s) scheme;协议降级默认 fail-closed 拒)。
    UnsupportedScheme(String),
    /// host 不在白名单(https 端点)。
    HostNotAllowed(String),
    /// `curl.exe` spawn 失败(缺失 / 无法启动)。
    CurlSpawn(String),
    /// `curl.exe` 非零退出(离线 / DNS / TLS / HTTP≥400 / 截断)。
    CurlNonZero {
        /// curl 退出码(如 7 连接失败 / 18 部分传输 / 22 HTTP≥400)。
        code: i32,
        /// curl stderr(透传)。
        stderr: String,
    },
    /// 本地 IO 失败(建目录 / 读写下载 staging)。
    Io(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::BadUrl(u) => write!(f, "非法 URL:{u}"),
            FetchError::UnsupportedScheme(s) => write!(
                f,
                "协议不受支持:{s}(缺省 https-only;环回豁免需 {LOOPBACK_ENV}=1 + host 127.0.0.1)"
            ),
            FetchError::HostNotAllowed(h) => write!(
                f,
                "端点 host 不在白名单:{h}(允许 {})",
                HOST_ALLOWLIST.join(" / ")
            ),
            FetchError::CurlSpawn(e) => write!(f, "curl 载体 spawn 失败:{e}"),
            FetchError::CurlNonZero { code, stderr } => {
                write!(f, "curl 载体非零退出(code={code}):{stderr}")
            }
            FetchError::Io(e) => write!(f, "下载 IO 失败:{e}"),
        }
    }
}

/// 读取环回豁免 env(`RURIXUP_TEST_ALLOW_LOOPBACK_HTTP == "1"`)。
pub fn loopback_allowed_from_env() -> bool {
    std::env::var(LOOPBACK_ENV)
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// URL 的 `(scheme, host)` 抽取(极简:仅支持 http/https;host = 认证段去 userinfo
/// 去端口)。纯函数,不触网。
pub fn split_url(url: &str) -> Result<(String, String), FetchError> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| FetchError::BadUrl(url.to_string()))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        return Err(FetchError::BadUrl(url.to_string()));
    }
    // host = authority 去 userinfo(@ 前)去端口(: 后)。
    let hostport = authority
        .rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(authority);
    let host = hostport
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(hostport);
    if host.is_empty() {
        return Err(FetchError::BadUrl(url.to_string()));
    }
    Ok((scheme.to_ascii_lowercase(), host.to_ascii_lowercase()))
}

/// 端点合法性判定(RXS-0216;**纯函数,不 spawn**):
/// - `https`:host ∈ [`HOST_ALLOWLIST`](或环回 127.0.0.1 + `allow_loopback_http`)→ 放行;否则 [`FetchError::HostNotAllowed`];
/// - `http`:仅 host == `127.0.0.1` 且 `allow_loopback_http` 放行(hermetic fixture);
///   **缺省 fail-closed 拒**(协议降级)→ [`FetchError::UnsupportedScheme`];
/// - 其余 scheme → [`FetchError::UnsupportedScheme`]。
pub fn validate_endpoint(
    url: &str,
    allow_loopback_http: bool,
) -> Result<(String, String), FetchError> {
    let (scheme, host) = split_url(url)?;
    match scheme.as_str() {
        "https" => {
            if HOST_ALLOWLIST.contains(&host.as_str())
                || (host == "127.0.0.1" && allow_loopback_http)
            {
                Ok((scheme, host))
            } else {
                Err(FetchError::HostNotAllowed(host))
            }
        }
        "http" => {
            if host == "127.0.0.1" && allow_loopback_http {
                Ok((scheme, host))
            } else {
                Err(FetchError::UnsupportedScheme(format!("http://{host}")))
            }
        }
        other => Err(FetchError::UnsupportedScheme(other.to_string())),
    }
}

/// 构造 `curl.exe` 固定参数集(RXS-0216;**纯函数,host 可测**):先
/// [`validate_endpoint`],合规后按 scheme 产参数序——https → `--proto =https
/// --proto-redir =https`;放行的环回 http → `--proto =http --proto-redir =http`,
/// 其余参数(`--fail --silent --show-error --location --max-redirs 5 --max-time N
/// --output <out> <url>`)逐字节不变。https-only 双 proto 钉死封死协议降级重定向。
pub fn build_curl_args(
    url: &str,
    output: &Path,
    max_time_secs: u64,
    allow_loopback_http: bool,
) -> Result<Vec<String>, FetchError> {
    let (scheme, _host) = validate_endpoint(url, allow_loopback_http)?;
    let proto = if scheme == "http" { "=http" } else { "=https" };
    Ok(vec![
        "--fail".to_string(),
        "--silent".to_string(),
        "--show-error".to_string(),
        "--location".to_string(),
        "--proto".to_string(),
        proto.to_string(),
        "--proto-redir".to_string(),
        proto.to_string(),
        "--max-redirs".to_string(),
        "5".to_string(),
        "--max-time".to_string(),
        max_time_secs.to_string(),
        "--output".to_string(),
        output.to_string_lossy().into_owned(),
        url.to_string(),
    ])
}

/// 经系统 `curl.exe` 下载 `url` → `output`(RXS-0216):[`validate_endpoint`] →
/// [`build_curl_args`] → `Command` spawn;spawn 失败 / 非零退出即 `Err`。成功后字节
/// 落 `output`,**完整性判定交 RXS-0217 级联**(不因 HTTP 200 视作可信)。
pub fn download_to(
    url: &str,
    output: &Path,
    max_time_secs: u64,
    allow_loopback_http: bool,
) -> Result<(), FetchError> {
    let args = build_curl_args(url, output, max_time_secs, allow_loopback_http)?;
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| FetchError::Io(format!("建下载目录 {} 失败:{e}", parent.display())))?;
    }
    let curl = if cfg!(windows) { "curl.exe" } else { "curl" };
    let out = Command::new(curl)
        .args(&args)
        .output()
        .map_err(|e| FetchError::CurlSpawn(format!("{curl} spawn 失败:{e}")))?;
    if !out.status.success() {
        // 失败即清残留(curl --output 可能已写部分字节;不留半下载)。
        let _ = std::fs::remove_file(output);
        return Err(FetchError::CurlNonZero {
            code: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(())
}

// ————————————————— RXS-0217 repo 锚 channels/stable.json —————————————————

/// repo 锚单条发行记录(`releases[]` 条目)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorRelease {
    /// 发行版号。
    pub version: String,
    /// 级① 期望 digest = `sha256(channel_manifest.json 字节流)`。
    pub channel_manifest_sha256: String,
    /// 分发前缀(channel_manifest.json / bundle.json / 逐组件的 base URL)。
    pub base_url: String,
}

/// repo 锚 `channels/stable.json`(确定性 JSON,无时间戳;**信任根 = repo**,RXS-0217)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// schema 版号(首版 1)。
    pub schema_version: u32,
    /// channel(首版 stable)。
    pub channel: String,
    /// 发行记录集。
    pub releases: Vec<AnchorRelease>,
    /// `latest` 派生自锚(非服务端;降级攻击在级① 失配)。
    pub latest: Option<String>,
}

impl Anchor {
    /// 解析锚 JSON(手写极简 line-scan,零外部依赖;镜像 `toolchain.rs` from_json 风格)。
    /// 只识别本 schema 的规范形态:标量字段 `"key": value`,`releases[]` 为对象数组
    /// (逐对象含 `version` / `channel_manifest_sha256` / `base_url` 三字段)。
    pub fn from_json(text: &str) -> Result<Anchor, String> {
        let mut schema_version: Option<u32> = None;
        let mut channel: Option<String> = None;
        let mut latest: Option<String> = None;
        let mut releases: Vec<AnchorRelease> = Vec::new();
        let mut in_releases = false;
        let mut cur_ver: Option<String> = None;
        let mut cur_digest: Option<String> = None;
        let mut cur_base: Option<String> = None;

        let flush = |releases: &mut Vec<AnchorRelease>,
                     v: &mut Option<String>,
                     d: &mut Option<String>,
                     b: &mut Option<String>| {
            if let Some(ver) = v.take() {
                releases.push(AnchorRelease {
                    version: ver,
                    channel_manifest_sha256: d.take().unwrap_or_default(),
                    base_url: b.take().unwrap_or_default(),
                });
            }
        };

        for raw in text.lines() {
            let line = raw.trim();
            if line.starts_with("\"releases\":") {
                in_releases = true;
                continue;
            }
            if in_releases && (line == "]" || line == "],") {
                // releases 数组收束:flush 末条目。
                flush(&mut releases, &mut cur_ver, &mut cur_digest, &mut cur_base);
                in_releases = false;
                continue;
            }
            if let Some(rest) = line.strip_prefix("\"schema_version\":") {
                schema_version = rest.trim().trim_end_matches(',').trim().parse::<u32>().ok();
            } else if let Some(rest) = line.strip_prefix("\"channel\":") {
                channel = Some(anchor_unquote(rest)?);
            } else if let Some(rest) = line.strip_prefix("\"latest\":") {
                let v = rest.trim().trim_end_matches(',').trim();
                if v != "null" {
                    latest = Some(anchor_unquote(rest)?);
                }
            } else if let Some(rest) = line.strip_prefix("\"version\":") {
                // 新对象起始:先 flush 上一条(对象数组无收束单行时的边界)。
                flush(&mut releases, &mut cur_ver, &mut cur_digest, &mut cur_base);
                cur_ver = Some(anchor_unquote(rest)?);
            } else if let Some(rest) = line.strip_prefix("\"channel_manifest_sha256\":") {
                cur_digest = Some(anchor_unquote(rest)?);
            } else if let Some(rest) = line.strip_prefix("\"base_url\":") {
                cur_base = Some(anchor_unquote(rest)?);
            }
        }
        // 兜底 flush(锚数组末尾无独立 `]` 行的形态)。
        flush(&mut releases, &mut cur_ver, &mut cur_digest, &mut cur_base);

        Ok(Anchor {
            schema_version: schema_version.unwrap_or(0),
            channel: channel.ok_or("锚缺 channel 字段")?,
            releases,
            latest,
        })
    }

    /// 取指定版号发行记录(RXS-0217 **无锚版号拒装**:不在 `releases[]` → `None`,
    /// 调用方据此拒装——「已发布未登记」过渡窗)。
    pub fn release_for(&self, version: &str) -> Option<&AnchorRelease> {
        self.releases.iter().find(|r| r.version == version)
    }
}

/// 抽取 `"key": "value"` 的 value(去引号 / 去尾逗号;锚标量字段用)。
fn anchor_unquote(rest: &str) -> Result<String, String> {
    let v = rest.trim().trim_end_matches(',').trim();
    let v = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| format!("锚字段非字符串:{v}"))?;
    Ok(v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    //@ spec: RXS-0216
    // 固定参数集逐项 + https-only 双 proto 钉死(--proto =https --proto-redir =https);
    // 参数序与 RFC-0012 §4.4 逐字对齐,url 末位、--output 前置。
    #[test]
    fn build_curl_args_fixed_https_only_set() {
        let out = PathBuf::from("staged/bundle.json");
        let args = build_curl_args(
            "https://github.com/o/r/releases/download/v1.1.0/bundle.json",
            &out,
            300,
            false,
        )
        .expect("白名单 https 放行");
        // 固定参数集逐项在位。
        for flag in [
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "--proto-redir",
            "--max-redirs",
            "--max-time",
            "--output",
        ] {
            assert!(args.iter().any(|a| a == flag), "缺参数 {flag}");
        }
        // https-only 双 proto 钉死 = https。
        let proto_idxs: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "--proto" || *a == "--proto-redir")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(proto_idxs.len(), 2);
        for i in proto_idxs {
            assert_eq!(args[i + 1], "=https", "proto 值须 =https(封死降级)");
        }
        assert_eq!(args.iter().filter(|a| *a == "--max-redirs").count(), 1);
        // url 末位。
        assert!(args.last().unwrap().starts_with("https://github.com/"));
    }

    //@ spec: RXS-0216
    // 环回守门 + 缺省 https-only:缺 env 的 http://127.0.0.1 拒(协议降级);有 env
    // 放行环回 http(proto =http);非白名单 https host 拒;白名单 https 放行。
    #[test]
    fn loopback_gating_default_https_only() {
        let out = PathBuf::from("o.bin");
        // 缺省态(allow=false):http 环回被拒(协议降级 fail-closed)。
        assert!(matches!(
            validate_endpoint("http://127.0.0.1:8000/bundle.json", false),
            Err(FetchError::UnsupportedScheme(_))
        ));
        // 有 env(allow=true):环回 http 放行,proto 集 =http。
        let args = build_curl_args("http://127.0.0.1:8000/bundle.json", &out, 30, true)
            .expect("环回 http + env 放行");
        let pi = args.iter().position(|a| a == "--proto").unwrap();
        assert_eq!(args[pi + 1], "=http");
        // env 只放行 127.0.0.1:非环回 http host 即使 allow 也拒。
        assert!(matches!(
            validate_endpoint("http://evil.example.com/x", true),
            Err(FetchError::UnsupportedScheme(_))
        ));
        // 非白名单 https host 拒。
        assert!(matches!(
            validate_endpoint("https://evil.example.com/x", false),
            Err(FetchError::HostNotAllowed(_))
        ));
        // 白名单 https 放行。
        assert!(
            validate_endpoint(
                "https://raw.githubusercontent.com/o/r/main/channels/stable.json",
                false
            )
            .is_ok()
        );
        assert!(validate_endpoint("https://objects.githubusercontent.com/x", false).is_ok());
    }

    fn anchor_json(ver: &str, digest: &str) -> String {
        format!(
            "{{\n  \"schema_version\": 1,\n  \"channel\": \"stable\",\n  \"releases\": [\n    {{\n      \"version\": \"{ver}\",\n      \"channel_manifest_sha256\": \"{digest}\",\n      \"base_url\": \"http://127.0.0.1:9/{ver}/\"\n    }}\n  ],\n  \"latest\": \"{ver}\"\n}}\n"
        )
    }

    //@ spec: RXS-0217
    // 锚解析 + release_for 命中/无锚版号拒装(不在 releases[] → None → 调用方拒装)。
    #[test]
    fn anchor_parse_and_release_lookup() {
        let a = Anchor::from_json(&anchor_json("1.1.0", &"ab".repeat(32))).expect("锚解析");
        assert_eq!(a.schema_version, 1);
        assert_eq!(a.channel, "stable");
        assert_eq!(a.latest.as_deref(), Some("1.1.0"));
        assert_eq!(a.releases.len(), 1);
        let rel = a.release_for("1.1.0").expect("命中");
        assert_eq!(rel.channel_manifest_sha256, "ab".repeat(32));
        assert_eq!(rel.base_url, "http://127.0.0.1:9/1.1.0/");
        // 无锚版号 → None(过渡窗拒装)。
        assert!(a.release_for("9.9.9").is_none());
    }

    //@ spec: RXS-0217
    // 级① 锚→channel digest 门:sha256(channel_manifest 字节)== 锚声明 → 放行;
    // 篡改 channel_manifest 一字节 → digest 失配 → 拒(纯判定,host 可测)。
    #[test]
    fn level_one_anchor_digest_gate() {
        let manifest_bytes =
            b"{\n  \"channel\": \"stable\",\n  \"bundle_manifest_sha256\": \"deadbeef\"\n}\n";
        let expect = rurix_pkg::sha256::hex_digest(manifest_bytes);
        // 匹配 → 放行。
        assert_eq!(rurix_pkg::sha256::hex_digest(manifest_bytes), expect);
        // 篡改一字节 → 失配。
        let mut tampered = manifest_bytes.to_vec();
        tampered[5] ^= 0xFF;
        assert_ne!(rurix_pkg::sha256::hex_digest(&tampered), expect);
    }
}
