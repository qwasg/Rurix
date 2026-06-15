"""traceability 矩阵工具首版(契约 G-M1-4;10 §4:每条款 ≥1 测试锚定)。

用法:py -3 ci/trace_matrix.py [--check]
  默认:重新生成 conformance/traceability_matrix.json + .md 并校验全锚定;
  --check:只校验(不写文件),入库矩阵与现状不一致或存在未锚定条款即 FAIL。

条款源:spec/*.md 的 `### RXS-####` 标题。
锚定源(`//@ spec: RXS-####, ...` 注释行):
  - conformance/**/*.rx(语法样例集)
  - tests/ui/**/*.rx(UI golden 样例)
  - src/rurixc/**/*.rs(单测锚定注释)
  - src/rurix-rt/**/*.rs(M4.3 运行时单测/真跑测试锚定注释)
存在未锚定条款时退出码 1(budget_eval 的 m1.counter.spec_clause_test_anchoring
消费本工具产物 JSON 的 clauses 字段)。
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MATRIX_JSON = ROOT / "conformance" / "traceability_matrix.json"
MATRIX_MD = ROOT / "conformance" / "traceability_matrix.md"

CLAUSE_RE = re.compile(r"^###\s+(RXS-\d{4})\b", re.MULTILINE)
ANCHOR_LINE_RE = re.compile(r"//@\s*spec:\s*(.+)")
CLAUSE_ID_RE = re.compile(r"RXS-\d{4}")


def parse_clauses(spec_texts: dict[str, str]) -> dict[str, str]:
    """spec 文件文本 → {条款号: 所在文件}。重复定义即异常(编号全局唯一,10 §9.5)。"""
    clauses: dict[str, str] = {}
    for path, text in sorted(spec_texts.items()):
        for cid in CLAUSE_RE.findall(text):
            if cid in clauses:
                raise ValueError(f"条款号重复定义: {cid}({clauses[cid]} 与 {path})")
            clauses[cid] = path
    return clauses


def collect_anchors(test_texts: dict[str, str]) -> dict[str, list[str]]:
    """测试文件文本 → {条款号: [锚定文件(去重排序)]}。"""
    anchors: dict[str, set[str]] = {}
    for path, text in test_texts.items():
        for m in ANCHOR_LINE_RE.finditer(text):
            for cid in CLAUSE_ID_RE.findall(m.group(1)):
                anchors.setdefault(cid, set()).add(path)
    return {cid: sorted(paths) for cid, paths in anchors.items()}


def build_matrix(
    clauses: dict[str, str], anchors: dict[str, list[str]]
) -> tuple[dict, list[str], list[str]]:
    """返回 (矩阵文档, 未锚定条款, 幽灵锚定[引用不存在条款])。"""
    matrix = {
        "schema_version": 1,
        "generated_by": "ci/trace_matrix.py",
        "description": "spec 条款 ↔ 测试锚定矩阵(G-M1-4;budget_eval 消费 clauses 字段,空列表 = 未锚定即 FAIL)",
        "clauses": {cid: anchors.get(cid, []) for cid in sorted(clauses)},
    }
    unanchored = [cid for cid in sorted(clauses) if not anchors.get(cid)]
    ghosts = [cid for cid in sorted(anchors) if cid not in clauses]
    return matrix, unanchored, ghosts


def render_md(matrix: dict, clause_files: dict[str, str]) -> str:
    lines = [
        "# spec 条款 ↔ 测试锚定矩阵(生成物,勿手改)",
        "",
        "> 生成:`py -3 ci/trace_matrix.py`(G-M1-4;每条款 ≥1 测试锚定,10 §4)。",
        "",
        "| 条款 | spec 文件 | 锚定测试数 | 锚定 |",
        "|---|---|---|---|",
    ]
    for cid, tests in matrix["clauses"].items():
        shown = ", ".join(f"`{t}`" for t in tests[:3])
        if len(tests) > 3:
            shown += f" …(+{len(tests) - 3})"
        lines.append(f"| {cid} | {clause_files[cid]} | {len(tests)} | {shown} |")
    lines.append("")
    return "\n".join(lines)


def gather_repo() -> tuple[dict[str, str], dict[str, str]]:
    spec_texts = {
        p.relative_to(ROOT).as_posix(): p.read_text(encoding="utf-8")
        for p in sorted((ROOT / "spec").glob("*.md"))
        if p.name != "README.md"
    }
    test_files: list[Path] = []
    test_files += sorted((ROOT / "conformance").glob("**/*.rx"))
    test_files += sorted((ROOT / "tests" / "ui").glob("**/*.rx"))
    test_files += sorted((ROOT / "src" / "rurixc").glob("**/*.rs"))
    test_files += sorted((ROOT / "src" / "rurix-rt").glob("**/*.rs"))
    # M6.1:rx CLI crate(子命令语义面锚定;src/rx 随 rx CLI 落地存在)
    test_files += sorted((ROOT / "src" / "rx").glob("**/*.rs"))
    test_texts = {
        p.relative_to(ROOT).as_posix(): p.read_text(encoding="utf-8") for p in test_files
    }
    return spec_texts, test_texts


def main() -> int:
    check_only = "--check" in sys.argv
    spec_texts, test_texts = gather_repo()
    clauses = parse_clauses(spec_texts)
    anchors = collect_anchors(test_texts)
    matrix, unanchored, ghosts = build_matrix(clauses, anchors)

    failures = []
    if unanchored:
        failures.append(f"未锚定条款({len(unanchored)}): {', '.join(unanchored)}")
    if ghosts:
        failures.append(f"幽灵锚定(引用不存在的条款号): {', '.join(ghosts)}")

    json_text = json.dumps(matrix, ensure_ascii=False, indent=2) + "\n"
    md_text = render_md(matrix, clauses)
    if check_only:
        if not MATRIX_JSON.is_file() or MATRIX_JSON.read_text(encoding="utf-8") != json_text:
            failures.append(
                "入库矩阵与现状不一致(运行 py -3 ci/trace_matrix.py 重新生成)"
            )
    else:
        MATRIX_JSON.write_text(json_text, encoding="utf-8")
        MATRIX_MD.write_text(md_text, encoding="utf-8")

    if failures:
        print("[trace_matrix] FAIL")
        for f in failures:
            print(f"  - {f}")
        return 1
    anchored_total = sum(1 for t in matrix["clauses"].values() if t)
    print(
        f"[trace_matrix] PASS ({anchored_total}/{len(clauses)} clauses anchored, "
        f"{len(test_texts)} test files scanned)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
