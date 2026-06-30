//! `rx doc`(M8.6,D-M8-6 / G-M8-6):从既有单一事实源确定性生成静态文档站。
//!
//! 判档(M8_CONTRACT §7,agent 裁定):文档站系对**既有** `spec/` 条款 / `registry/error_codes.json`
//! 错误码目录 / `conformance/traceability_matrix.json` 锚定矩阵的**工程化呈现**,纯工程、不触新规范面、
//! 不造裸条款;归口既有 CLI 分发与退出码条款 RXS-0083。错误码注册表自述「人类可读文档由工具生成」
//! (07 §5),`rx doc` 即该工具。
//!
//! 纪律:纯 safe Rust(`unsafe_code=deny` 维持);**不引新外部依赖**——JSON 解析复用
//! [`rurixc::tooling::json_util`](rurixc::tooling::json_util)(本仓既有无 serde 读取器)。生成确定性:
//! 内容仅取自单一事实源 + 稳定排序 + 产物不含 wall-clock/随机量,同输入两次落盘逐字节一致
//! (对齐 image-io / UC-03 demo 确定性帧先例,G-M7-1/G-M7-3)。
//@ spec: RXS-0083

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rurixc::tooling::json_util::{json_array_field, json_object_field, json_str_field};

/// 一条规范条款(`### RXS-#### 标题` + 条款体 + 源文件)。
struct Clause {
    id: String,
    title: String,
    body: String,
    source: String,
}

/// 一条错误码目录项(`registry/error_codes.json` entries 元素)。
struct ErrCode {
    id: String,
    title: String,
    message_key: String,
    status: String,
    introduced_in: String,
    spec_clauses: Vec<String>,
}

/// 生成统计(供 stdout 摘要)。
struct Summary {
    clauses: usize,
    errors: usize,
    pages: usize,
}

/// `rx doc [--root <dir>] [--out <dir>]`(RXS-0083 退出码:0 成功 / 1 生成失败 / 2 用法错误)。
pub fn run(args: &[String]) -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut root_arg: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return usage("`--out` 缺目录参数");
                };
                out = Some(PathBuf::from(v));
            }
            "--root" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return usage("`--root` 缺目录参数");
                };
                root_arg = Some(PathBuf::from(v));
            }
            s => return usage(&format!("无法识别的参数 `{s}`")),
        }
        i += 1;
    }

    let start = root_arg.unwrap_or_else(|| PathBuf::from("."));
    let Some(root) = locate_content_root(&start) else {
        eprintln!(
            "rx doc: error: 未找到内容根(从 {} 向上须含 spec/ 与 registry/error_codes.json)",
            start.display()
        );
        return ExitCode::from(1);
    };
    let out_dir = out.unwrap_or_else(|| root.join("target").join("doc"));

    match generate(&root, &out_dir) {
        Ok(sum) => {
            println!(
                "rx doc: 生成完成 → {}({} 条款 / {} 错误码 / {} 页)",
                out_dir.display(),
                sum.clauses,
                sum.errors,
                sum.pages
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rx doc: error: {e}");
            ExitCode::from(1)
        }
    }
}

/// 用法诊断(RX7003,7xxx 链接/工具链段位 rx CLI 复用,无新错误码)。
fn usage(detail: &str) -> ExitCode {
    eprintln!("rx doc: error[RX7003]: {detail}");
    eprintln!("usage: rx doc [--root <dir>] [--out <dir>]");
    ExitCode::from(2)
}

/// 从 `start` 向上定位含 `spec/` 与 `registry/error_codes.json` 的内容根。
fn locate_content_root(start: &Path) -> Option<PathBuf> {
    let mut dir = std::fs::canonicalize(start).ok()?;
    loop {
        if dir.join("spec").is_dir() && dir.join("registry").join("error_codes.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// 端到端:读单一事实源 → 渲染 → 落盘(确定性)。
fn generate(root: &Path, out_dir: &Path) -> Result<Summary, String> {
    let clauses = collect_clauses(&root.join("spec"))?;
    let errors = collect_error_codes(&root.join("registry").join("error_codes.json"))?;
    let matrix = read_to_string(&root.join("conformance").join("traceability_matrix.json"))?;
    let clauses_obj =
        json_object_field(&matrix, "clauses").ok_or("traceability_matrix.json 缺 clauses 对象")?;

    let pages = [
        ("index.html", render_index(&clauses, &errors)),
        ("spec.html", render_spec(&clauses)),
        ("errors.html", render_errors(&errors)),
        (
            "traceability.html",
            render_traceability(&clauses, clauses_obj),
        ),
    ];

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("无法创建输出目录 {}: {e}", out_dir.display()))?;
    for (name, html) in &pages {
        let p = out_dir.join(name);
        std::fs::write(&p, html).map_err(|e| format!("写 {} 失败: {e}", p.display()))?;
    }

    Ok(Summary {
        clauses: clauses.len(),
        errors: errors.len(),
        pages: pages.len(),
    })
}

// ───────────────────────── 内容源解析 ─────────────────────────

/// 扫描 `spec/*.md`(文件名排序),按 `### RXS-#### 标题` 切条款体(条款号排序)。
fn collect_clauses(spec_dir: &Path) -> Result<Vec<Clause>, String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(spec_dir)
        .map_err(|e| format!("读 spec/ 失败: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    files.sort();

    let mut clauses: Vec<Clause> = Vec::new();
    for f in &files {
        let text = read_to_string(f)?;
        let src = f
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_owned();
        parse_clauses_from(&text, &src, &mut clauses);
    }
    clauses.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(clauses)
}

/// 单文件条款切分:跟踪代码围栏;`### RXS-####` 起新条款,其余 md 标题(`# `~`###### `)结束当前条款。
fn parse_clauses_from(text: &str, src: &str, out: &mut Vec<Clause>) {
    let mut in_fence = false;
    let mut cur: Option<Clause> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            if let Some(c) = cur.as_mut() {
                c.body.push_str(line);
                c.body.push('\n');
            }
            continue;
        }
        if !in_fence {
            if let Some((id, title)) = parse_clause_header(line) {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
                cur = Some(Clause {
                    id,
                    title,
                    body: String::new(),
                    source: src.to_owned(),
                });
                continue;
            }
            if is_md_heading(line) {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
                continue;
            }
        }
        if let Some(c) = cur.as_mut() {
            c.body.push_str(line);
            c.body.push('\n');
        }
    }
    if let Some(c) = cur.take() {
        out.push(c);
    }
}

/// `### RXS-#### <标题>` → (id, 标题);非此形态 → None。
fn parse_clause_header(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("### ")?.strip_prefix("RXS-")?;
    if rest.len() < 4 {
        return None;
    }
    let (num, tail) = rest.split_at(4);
    if !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((format!("RXS-{num}"), tail.trim().to_owned()))
}

/// markdown ATX 标题(1~6 个 `#` 后跟空格)。
fn is_md_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    (1..=6).contains(&hashes) && line[hashes..].starts_with(' ')
}

/// 解析 `registry/error_codes.json` 的 entries(id 排序)。
fn collect_error_codes(path: &Path) -> Result<Vec<ErrCode>, String> {
    let text = read_to_string(path)?;
    let entries = json_array_field(&text, "entries").ok_or("error_codes.json 缺 entries 数组")?;
    let mut out: Vec<ErrCode> = Vec::new();
    for obj in array_top_level_elements(entries) {
        let Some(id) = json_str_field(obj, "id") else {
            continue;
        };
        let spec_clauses = json_array_field(obj, "spec_clauses")
            .map(array_top_level_strings)
            .unwrap_or_default();
        out.push(ErrCode {
            id,
            title: json_str_field(obj, "title").unwrap_or_default(),
            message_key: json_str_field(obj, "message_key").unwrap_or_default(),
            status: json_str_field(obj, "status").unwrap_or_default(),
            introduced_in: json_str_field(obj, "introduced_in").unwrap_or_default(),
            spec_clauses,
        });
    }
    if out.is_empty() {
        return Err("error_codes.json entries 为空(解析异常)".to_owned());
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn read_to_string(p: &Path) -> Result<String, String> {
    std::fs::read_to_string(p).map_err(|e| format!("读 {} 失败: {e}", p.display()))
}

// ── 极简 JSON 数组切分(复用 json_util 的字段读取,补足数组元素枚举)──

/// 在数组文本(含外层 `[]`)中按顶层逗号切元素原文(尊重字符串与括号嵌套)。
fn array_top_level_elements(arr: &str) -> Vec<&str> {
    let inner = strip_delims(arr, '[', ']');
    let mut parts: Vec<&str> = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    let mut start = 0usize;
    for (i, ch) in inner.char_indices() {
        if in_str {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(inner[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    let tail = inner[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts.into_iter().filter(|p| !p.is_empty()).collect()
}

/// 顶层字符串数组 → 去引号去转义的字符串向量。
fn array_top_level_strings(arr: &str) -> Vec<String> {
    array_top_level_elements(arr)
        .into_iter()
        .filter_map(unquote)
        .collect()
}

fn strip_delims(s: &str, open: char, close: char) -> &str {
    let t = s.trim();
    t.strip_prefix(open)
        .and_then(|x| x.strip_suffix(close))
        .map(str::trim)
        .unwrap_or(t)
}

fn unquote(s: &str) -> Option<String> {
    let t = s.trim();
    let mut chars = t.strip_prefix('"')?.chars();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                o => out.push(o),
            },
            '"' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

// ───────────────────────── 渲染(确定性 HTML) ─────────────────────────

const STYLE: &str = "body{font-family:system-ui,sans-serif;max-width:60rem;margin:2rem auto;padding:0 1rem;line-height:1.55}\
code,pre{font-family:ui-monospace,Consolas,monospace}\
pre{background:#f6f8fa;padding:.75rem 1rem;overflow:auto;white-space:pre-wrap}\
table{border-collapse:collapse;width:100%}th,td{border:1px solid #d0d7de;padding:.35rem .6rem;text-align:left;vertical-align:top}\
section{border-top:1px solid #eaecef;padding-top:.5rem;margin-top:1.5rem}\
.src{color:#57606a;font-size:.85rem}nav a{margin-right:1rem}";

fn page(title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"zh\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{t}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n\
<nav><a href=\"index.html\">首页</a><a href=\"spec.html\">规范</a>\
<a href=\"errors.html\">错误码</a><a href=\"traceability.html\">锚定矩阵</a></nav>\n\
{body}\n\
<footer class=\"src\">由 <code>rx doc</code> 从 spec/ + registry/error_codes.json + \
conformance/traceability_matrix.json 确定性生成。</footer>\n</body>\n</html>\n",
        t = html_escape(title),
    )
}

fn render_index(clauses: &[Clause], errors: &[ErrCode]) -> String {
    let body = format!(
        "<h1>Rurix 文档站</h1>\n\
<p>本站由 <code>rx doc</code> 从既有单一事实源工程化呈现(规范 / 错误码 / 条款锚定),确定性可复现。</p>\n\
<ul>\n\
<li><a href=\"spec.html\">规范条款</a>(共 {nc} 条,RXS-####)</li>\n\
<li><a href=\"errors.html\">错误码索引</a>(共 {ne} 条,RX####)</li>\n\
<li><a href=\"traceability.html\">条款 ↔ 测试锚定矩阵</a></li>\n\
</ul>",
        nc = clauses.len(),
        ne = errors.len(),
    );
    page("Rurix 文档站", &body)
}

fn render_spec(clauses: &[Clause]) -> String {
    let mut body = String::from("<h1>Rurix 规范条款</h1>\n<ul class=\"toc\">\n");
    for c in clauses {
        body.push_str(&format!(
            "<li><a href=\"#{id}\">{id} {title}</a></li>\n",
            id = html_escape(&c.id),
            title = html_escape(&c.title),
        ));
    }
    body.push_str("</ul>\n");
    for c in clauses {
        body.push_str(&format!(
            "<section id=\"{id}\">\n<h2>{id} {title}</h2>\n\
<p class=\"src\">来源:spec/{src}</p>\n<pre>{bodytext}</pre>\n</section>\n",
            id = html_escape(&c.id),
            title = html_escape(&c.title),
            src = html_escape(&c.source),
            bodytext = html_escape(c.body.trim_end()),
        ));
    }
    page("Rurix 规范条款", &body)
}

fn render_errors(errors: &[ErrCode]) -> String {
    let mut body = String::from(
        "<h1>Rurix 错误码索引</h1>\n\
<p>段位 0–7(词法/语法·名称/模块·类型·着色/地址空间·借用/生命周期·const eval·codegen/目标·链接/工具链);\
含义冻结、只增不改(07 §5 / 10 §6)。</p>\n\
<table>\n<thead><tr><th>错误码</th><th>含义</th><th>message-key</th><th>状态</th><th>引入</th><th>条款</th></tr></thead>\n<tbody>\n",
    );
    for e in errors {
        let clauses = e
            .spec_clauses
            .iter()
            .map(|c| format!("<a href=\"spec.html#{c}\">{c}</a>", c = html_escape(c)))
            .collect::<Vec<_>>()
            .join(", ");
        body.push_str(&format!(
            "<tr id=\"{id}\"><td><code>{id}</code></td><td>{title}</td><td><code>{mk}</code></td>\
<td>{status}</td><td>{intro}</td><td>{clauses}</td></tr>\n",
            id = html_escape(&e.id),
            title = html_escape(&e.title),
            mk = html_escape(&e.message_key),
            status = html_escape(&e.status),
            intro = html_escape(&e.introduced_in),
        ));
    }
    body.push_str("</tbody>\n</table>\n");
    page("Rurix 错误码索引", &body)
}

fn render_traceability(clauses: &[Clause], clauses_obj: &str) -> String {
    let mut body = String::from(
        "<h1>条款 ↔ 测试锚定矩阵</h1>\n\
<p>每条 RXS 条款的测试/语料锚定(来源 conformance/traceability_matrix.json;G-M1-4 / G-M8-7)。</p>\n",
    );
    for c in clauses {
        let anchors = json_array_field(clauses_obj, &c.id)
            .map(array_top_level_strings)
            .unwrap_or_default();
        body.push_str(&format!(
            "<section><h3 id=\"trace-{id}\">{id} <span class=\"src\">{title}</span></h3>\n",
            id = html_escape(&c.id),
            title = html_escape(&c.title),
        ));
        if anchors.is_empty() {
            body.push_str("<p class=\"src\">(未锚定)</p>\n");
        } else {
            body.push_str("<ul>\n");
            for a in &anchors {
                body.push_str(&format!("<li><code>{}</code></li>\n", html_escape(a)));
            }
            body.push_str("</ul>\n");
        }
        body.push_str("</section>\n");
    }
    page("条款 ↔ 测试锚定矩阵", &body)
}

fn html_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c => o.push(c),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clause_header_parse() {
        assert_eq!(
            parse_clause_header("### RXS-0083 rx CLI 总入口与子命令分发"),
            Some((
                "RXS-0083".to_owned(),
                "rx CLI 总入口与子命令分发".to_owned()
            ))
        );
        assert_eq!(parse_clause_header("### 普通三级标题"), None);
        assert_eq!(parse_clause_header("#[test] 不是标题"), None);
    }

    #[test]
    fn heading_excludes_attribute() {
        assert!(is_md_heading("## 范围"));
        assert!(is_md_heading("### RXS-0001 x"));
        assert!(!is_md_heading("#[test]"));
        assert!(!is_md_heading("普通文本"));
    }

    #[test]
    fn clause_body_stops_at_next_heading_and_keeps_fenced_hashes() {
        let md = "### RXS-0001 甲\n正文一\n```rust\n#[test] fn t(){}\n```\n### RXS-0002 乙\n正文二\n## 节\n尾\n";
        let mut out = Vec::new();
        parse_clauses_from(md, "x.md", &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "RXS-0001");
        assert!(out[0].body.contains("#[test] fn t(){}")); // 围栏内 # 不断条款
        assert_eq!(out[1].id, "RXS-0002");
        assert!(out[1].body.contains("正文二") && !out[1].body.contains("尾")); // ## 节 截断
    }

    #[test]
    fn json_array_element_split() {
        let arr = r#"[{"id":"RX0001","t":"a,b"},{"id":"RX0002"}]"#;
        let elems = array_top_level_elements(arr);
        assert_eq!(elems.len(), 2);
        assert_eq!(json_str_field(elems[0], "id").as_deref(), Some("RX0001"));
        assert_eq!(json_str_field(elems[0], "t").as_deref(), Some("a,b")); // 串内逗号不切
    }

    #[test]
    fn json_string_array() {
        assert_eq!(
            array_top_level_strings(r#"["RXS-0001", "RXS-0004"]"#),
            vec!["RXS-0001".to_owned(), "RXS-0004".to_owned()]
        );
    }

    #[test]
    fn html_escape_basic() {
        assert_eq!(html_escape("<a>&\"'"), "&lt;a&gt;&amp;&quot;&#39;");
    }
}
