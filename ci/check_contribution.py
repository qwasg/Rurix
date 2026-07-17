#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""贡献校验 CI 阻断门(G1.4 / MR-0003;CPU-only,check_ 守卫风格)。

兑现 10 §7「开源后 CI 自动阻断缺 provenance / 验证输出 / 条款号的 PR」(D-406)。
扫描 PR 范围(base..HEAD)的每个**非 merge commit**,三类缺项即红(反 YAML-only):

1. **Provenance**:每个 commit 含 `Assisted-by: <tool>:<model>` 或 `Co-Authored-By:`
   trailer(仓库既有约定 + AGENTS 硬规则 2)。
2. **条款号**:触 `src/**/*.rs` 或 `spec/**/*.md`(README 除外)的语义改动,须在 commit
   body / 新增 diff 行(`//@ spec: RXS-####`)/ 关联 `rfcs/*.md` 之一引用条款号或
   deferred/RFC 编号(`RXS-####` / `RD-###` / `RFC-####` / `MR-####`);纯文档/纯配置/
   纯 CI 改动豁免(硬规则 7)。
3. **验证强制**:触 `src/**/*.rs` 的 commit body 含验证标记(`Validation:` / `验证:` /
   `cargo test|build|clippy|fmt` / `ci/*.py` / `conformance` / `pytest`;数字必须来自命令
   输出,硬规则 3/10)。

另加一项 **PR 范围文件级**检查(非 per-commit,D-409 Proposed,10 §3 / §7):

4. **Full RFC 对抗性评审记录**:PR 范围(base..HEAD)内新增/修改的 `rfcs/NNNN-*.md`(Full RFC)
   须含**非空**「对抗性评审记录」段,且段内评审者 provenance(`Assisted-by: <tool>:<model>`)
   至少一个 **≠ 段外(起草)provenance**——评审 provenance ≠ 起草 provenance(硬规则 2 可机验),
   补自提自批单环。Mini-RFC(`mini-*.md`)不在强制面(轻量,模板 §7.1)。**能力边界(诚实)**:
   本门只校验**当前树内**该段存在性与 provenance 区分度,不核验评审内容质量、不枚举其它未合分支;
   模板占位 span(〈…〉/<…>)剥离后视同空段。

base 解析(优先级:命令行参数 > GITHUB_BASE_REF > origin/main > main):贡献门是 **PR 范围**
语义,默认基准取 `origin/main`(只校验本 PR 自己的 commit,不回溯已合入历史)——区别于
`check_guardrails.py` 的字节级基准 `m8-closed`。空范围(如 push 后)→ PASS。

**行为(与既有一致,不改总体)**:四项均为 **advisory**——`main()` 恒返回 0,缺项打印 finding
但**不阻断**合入(agent 完全自主化,10 §7 v2.0 / AGENTS v3.0);检测逻辑保留以维持可审计性。

check_ 守卫风格:**不分配错误码**(07 §5)、**不写 evidence**、**不接 budget counter**;
内置 `red_self_test()` 反 YAML-only(合成缺项 commit / 缺段 RFC 断言判红、齐备样本断言判绿)。

用法: py -3 ci/check_contribution.py [base_ref]
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# —— 规则正则 ——
# Provenance trailer:`Assisted-by: tool:model` 或 `Co-Authored-By: name <email>`。
PROVENANCE_RE = re.compile(
    r"(?im)^\s*(?:Assisted-by:\s*\S+:\S+|Co-Authored-By:\s*.+\S)\s*$"
)
# 条款号 / deferred / RFC / Mini-RFC 编号(语义改动须引用其一)。
CLAUSE_ID_RE = re.compile(r"\b(?:RXS-\d{4}|RD-\d{3}|RFC-\d{4}|MR-\d{4})\b")
# 验证标记(src 改动 commit body 须含其一)。
VERIFY_RE = re.compile(
    r"(?im)(?:Validation:|验证[:：]|cargo\s+(?:test|build|clippy|fmt)\b|"
    r"ci/[\w-]+\.py|conformance|pytest)"
)
# 语义改动文件:src 下 .rs 或 spec 下 .md(README 除外,与 trace_matrix 口径一致)。
SRC_RS_RE = re.compile(r"^src/.+\.rs$")
SPEC_MD_RE = re.compile(r"^spec/.+\.md$")

# —— 规则 4:Full RFC 对抗性评审记录门(D-409 Proposed) ——
# Full RFC 文件 rfcs/NNNN-*.md(mini-*.md / README / TEMPLATE 首字符非数字,天然排除)。
FULL_RFC_RE = re.compile(r"^rfcs/\d{4}-.+\.md$")
# 「对抗性评审记录」段标题(任意 # 级;`.` 不跨行,只匹配标题行)。
ADVERSARIAL_HEADING_RE = re.compile(r"(?m)^(#{1,6})\s+.*对抗性评审记录")
# Assisted-by trailer 中的 tool:model token(锚定 trailer,避免误配散落冒号)。
ASSISTED_BY_TOKEN_RE = re.compile(
    r"(?i)Assisted-by:\s*([A-Za-z0-9][\w.+-]*:[A-Za-z0-9][\w.+-]*)"
)
# 模板占位 span(填写前残留);校验前剥离,使『仅占位』段等价于空段。
PLACEHOLDER_SPAN_RE = re.compile(r"〈[^〉]*〉|<[^>\n]*>")


@dataclass
class CommitRecord:
    """单个 commit 的可校验快照(git 采集层与 red 自检层共用)。"""

    sha: str
    message: str
    files: list[str]
    added_text: str = ""  # 新增 diff 行(去 +++ 头)拼接,供条款号检测

    @property
    def short(self) -> str:
        return self.sha[:10] if self.sha else "<synthetic>"


def _touches_semantic(files: list[str]) -> bool:
    return any(
        SRC_RS_RE.match(f) or (SPEC_MD_RE.match(f) and not f.endswith("/README.md"))
        for f in files
    )


def _touches_src_rs(files: list[str]) -> bool:
    return any(SRC_RS_RE.match(f) for f in files)


def _touches_rfcs(files: list[str]) -> bool:
    return any(f.startswith("rfcs/") and f.endswith(".md") for f in files)


def check_commit(rec: CommitRecord) -> list[str]:
    """返回该 commit 的缺项列表(空 = 通过)。纯函数,red 自检直接喂合成记录。"""
    problems: list[str] = []

    # 规则 1:Provenance trailer。
    if not PROVENANCE_RE.search(rec.message):
        problems.append(
            "缺 provenance trailer(须含 `Assisted-by: <tool>:<model>` 或 "
            "`Co-Authored-By:`,D-406 / 硬规则 2)"
        )

    # 规则 2:语义改动须引用条款号 / deferred / RFC 编号。
    if _touches_semantic(rec.files):
        has_id = bool(
            CLAUSE_ID_RE.search(rec.message)
            or CLAUSE_ID_RE.search(rec.added_text)
            or _touches_rfcs(rec.files)
        )
        if not has_id:
            problems.append(
                "语义改动(src/*.rs 或 spec/*.md)未引用条款号"
                "(commit body / `//@ spec: RXS-####` 注释 / 关联 rfcs/*.md 之一,硬规则 7)"
            )

    # 规则 3:src 改动须附验证标记。
    if _touches_src_rs(rec.files) and not VERIFY_RE.search(rec.message):
        problems.append(
            "src 改动 commit body 缺验证标记(`Validation:` / `验证:` / "
            "`cargo test` / `ci/*.py` 等,硬规则 3/10)"
        )

    return problems


# ———————————— 规则 4:Full RFC 对抗性评审记录(纯函数) ————————————


def _strip_placeholders(text: str) -> str:
    """剥离模板占位 span(〈…〉/<…>),使『仅占位』段等价于空段。"""
    return PLACEHOLDER_SPAN_RE.sub(" ", text)


def _adversarial_section_body(text: str) -> str | None:
    """返回「对抗性评审记录」段正文(标题之后至下一同级/更高级标题前);无该段 → None。"""
    m = ADVERSARIAL_HEADING_RE.search(text)
    if m is None:
        return None
    level = len(m.group(1))
    body: list[str] = []
    for line in text[m.end():].splitlines():
        hm = re.match(r"^(#{1,6})\s", line)
        if hm and len(hm.group(1)) <= level:
            break  # 进入下一节
        body.append(line)
    return "\n".join(body)


def check_rfc_adversarial_review(text: str) -> list[str]:
    """Full RFC 对抗性评审记录校验(纯函数,pytest / red 自检直接喂文本)。

    要求(D-409 Proposed,10 §3 / §7):存在非空「对抗性评审记录」段,且段内评审者
    provenance(`Assisted-by: <tool>:<model>`)至少一个 **≠ 段外(起草)provenance**——
    评审 provenance ≠ 起草 provenance(硬规则 2 可机验)。模板占位 span 剥离后视同空段。
    返回缺项列表(空 = 通过)。诚实边界:只看当前文本,不核评审质量、不枚举其它分支。
    """
    problems: list[str] = []
    body = _adversarial_section_body(text)
    if body is None:
        problems.append(
            "缺「对抗性评审记录」段(Full RFC 强制,D-409;由 ≠ 起草 provenance 的工具/模型评审)"
        )
        return problems
    reviewer_tokens = set(ASSISTED_BY_TOKEN_RE.findall(_strip_placeholders(body)))
    # 起草 provenance = 段外(Header + 其余节)全部 Assisted-by token。
    outside = _strip_placeholders(text.replace(body, "", 1))
    drafting_tokens = set(ASSISTED_BY_TOKEN_RE.findall(outside))
    if not reviewer_tokens:
        problems.append(
            "「对抗性评审记录」段为空/仅占位"
            "(未见真实评审者 provenance `Assisted-by: <tool>:<model>`)"
        )
    elif not (reviewer_tokens - drafting_tokens):
        problems.append(
            "「对抗性评审记录」段评审 provenance 未与起草区分"
            "(评审 provenance 须 ≠ 起草 provenance,D-409 / 硬规则 2)"
        )
    return problems


def changed_full_rfcs(base: str) -> list[str]:
    """PR 范围(base..HEAD)内被改动的 Full RFC 文件路径(去重排序;删除项由调用侧按存在性跳过)。"""
    out = git("diff", "--name-only", f"{base}..HEAD").stdout
    return sorted({f.strip() for f in out.splitlines() if FULL_RFC_RE.match(f.strip())})


# ——————————————————————— git 采集层 ———————————————————————


def git(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=False
    )


def rev_exists(ref: str) -> bool:
    return git("rev-parse", "--verify", "--quiet", ref).returncode == 0


def resolve_base() -> str | None:
    if len(sys.argv) > 1:
        return sys.argv[1]
    gh_base = os.environ.get("GITHUB_BASE_REF")
    if gh_base:
        cand = f"origin/{gh_base}"
        return cand if rev_exists(cand) else gh_base
    # PR 范围语义:默认基准取 origin/main(只校验本 PR 自己的 commit)。
    for cand in ("origin/main", "main"):
        if rev_exists(cand):
            return cand
    return None


def collect_commits(base: str) -> list[CommitRecord]:
    out = git("log", "--no-merges", "--format=%H", f"{base}..HEAD").stdout
    shas = [s for s in out.splitlines() if s.strip()]
    records: list[CommitRecord] = []
    for sha in shas:
        message = git("log", "-1", "--format=%B", sha).stdout
        files = [
            f for f in git("diff-tree", "--no-commit-id", "--name-only", "-r", sha).stdout.splitlines()
            if f.strip()
        ]
        added = [
            ln[1:]
            for ln in git("show", "--format=", "--unified=0", sha).stdout.splitlines()
            if ln.startswith("+") and not ln.startswith("+++")
        ]
        records.append(
            CommitRecord(sha=sha, message=message, files=files, added_text="\n".join(added))
        )
    return records


# ——————————————————————— red 自检 ———————————————————————


def red_self_test() -> None:
    """反 YAML-only:合成缺项 / 齐备 commit,断言门能区分红绿。门失效即红。"""
    # (a) 缺 provenance + 缺条款 + 缺验证(触 src/*.rs)→ 应判红(≥3 问题)。
    bad = CommitRecord(
        sha="",
        message="feat: tweak geometry kernel\n",
        files=["src/rurix-geometry/src/lib.rs"],
        added_text="fn foo() {}\n",
    )
    bad_problems = check_commit(bad)
    if len(bad_problems) < 3:
        _fail(
            f"red 自检失败:缺项 commit 未被全数识别(检出 {len(bad_problems)}/3,门失效):"
            f"{bad_problems}"
        )
    # (b) 齐备(provenance + 条款 + 验证)→ 应判绿(0 问题)。
    good = CommitRecord(
        sha="",
        message=(
            "feat(geometry): add bvh node\n\n"
            "Validation: cargo test -p rurix-geometry PASS\n"
            "//@ spec: RXS-0113 covered\n\n"
            "Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>\n"
            "Assisted-by: claude-code:claude-opus-4-8\n"
        ),
        files=["src/rurix-geometry/src/lib.rs"],
        added_text="//@ spec: RXS-0113\n",
    )
    good_problems = check_commit(good)
    if good_problems:
        _fail(f"red 自检失败:齐备 commit 被误判有缺项(门过严):{good_problems}")

    # —— 规则 4 的红绿自检(合成 Full RFC 文本) ——
    rfc_header = (
        "# RFC-0099 — 合成\n\n"
        "| 字段 | 值 |\n|---|---|\n"
        "| Provenance | `Assisted-by: claude-code:claude-opus-4-8` |\n\n"
        "## 9. 未决问题\n\n〈占位〉\n"
    )
    # (c) 缺整段 → 应判红。
    if not check_rfc_adversarial_review(rfc_header):
        _fail("red 自检失败:缺「对抗性评审记录」段的 Full RFC 未被判红")
    # (d) 有段但仅占位(评审 provenance 未填)→ 应判红。
    rfc_placeholder = rfc_header + (
        "## 9.1 对抗性评审记录\n\n"
        "| 评审者 provenance | `Assisted-by: <评审 tool>:<评审 model>` |\n"
        "| F1 | 〈…〉 | 〈low〉 | 〈采纳/驳回〉 |\n"
    )
    if not check_rfc_adversarial_review(rfc_placeholder):
        _fail("red 自检失败:仅占位的对抗性评审段未被判红")
    # (e) 有段但评审 provenance == 起草 → 应判红。
    rfc_same = rfc_header + (
        "## 9.1 对抗性评审记录\n\n"
        "| 评审者 provenance | `Assisted-by: claude-code:claude-opus-4-8` |\n"
        "| F1 | 边界 | low | 采纳:改 §4 |\n"
    )
    if not check_rfc_adversarial_review(rfc_same):
        _fail("red 自检失败:评审 provenance 未与起草区分的 Full RFC 未被判红")
    # (f) 有段且评审 provenance ≠ 起草 → 应判绿。
    rfc_ok = rfc_header + (
        "## 9.1 对抗性评审记录\n\n"
        "| 评审者 provenance | `Assisted-by: codex:gpt-5` |\n"
        "| F1 | 边界 | med | 采纳:改 §4 |\n"
    )
    ok_problems = check_rfc_adversarial_review(rfc_ok)
    if ok_problems:
        _fail(f"red 自检失败:合规对抗性评审段被误判(门过严):{ok_problems}")


def _fail(msg: str) -> None:
    print(f"[check_contribution] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    red_self_test()

    base = resolve_base()
    if base is None or not rev_exists(base):
        print(
            "[check_contribution] PASS(无法解析基准 ref,跳过 PR 范围扫描;"
            "red 自检已过)"
        )
        return 0

    records = collect_commits(base)

    failures: list[str] = []
    for rec in records:
        for p in check_commit(rec):
            subject = rec.message.splitlines()[0] if rec.message.strip() else "<空提交信息>"
            failures.append(f"{rec.short} 「{subject}」: {p}")

    # 规则 4:PR 范围内改动的 Full RFC 须含合规「对抗性评审记录」段(D-409 Proposed)。
    rfc_paths = changed_full_rfcs(base)
    for path in rfc_paths:
        fpath = ROOT / path
        if not fpath.exists():
            continue  # 删除/重命名away → 跳过(存在性由树决定)
        for p in check_rfc_adversarial_review(fpath.read_text(encoding="utf-8")):
            failures.append(f"{path}(对抗性评审 D-409): {p}")

    if failures:
        # agent 完全自主化（10 §7 v2.0 / AGENTS v3.0）:provenance / 条款号 / 验证标记 /
        # RFC 对抗性评审 均降级为 advisory 审计输出,不阻断合入。检测逻辑保留以维持可审计性。
        print(
            f"[check_contribution] ADVISORY(base={base},{len(records)} commit + "
            f"{len(rfc_paths)} Full RFC 扫描,不阻断)"
        )
        for f in failures:
            print(f"  - {f}")
        print(
            "  说明:agent 完全自主模式下 provenance/条款号/验证/对抗性评审为建议项,不阻断合入。"
            "见 CONTRIBUTING.md / 10 §7 v2.0 / 13 D-409。"
        )
        return 0

    print(
        f"[check_contribution] PASS(base={base},{len(records)} 非 merge commit + "
        f"{len(rfc_paths)} Full RFC 全过:provenance + 条款号 + 验证 + 对抗性评审)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
