#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 对照报告一致性核(**步骤 75**;EI1.5 / RFC-0014 Part B;条款 RXS-0263 / RXS-0264 /
RXS-0265;验收门 **G-EI1-5**)。

**check_* 守卫风格,纯 host 恒跑,零 GPU、零工具链、零网络**:不分配错误码、不写 evidence、
**不写 budget counter**(CI_GATES §2 步骤 75 行明记)。与步骤 73(`ci/uc05_invariant_gate.py`,
拦截面:逐条 tier 断言 + 真编译门 + rhi.rs 库单测)**分工不重复造轮** —— 本门专注**报告面**:

  A. **schema 门**:`evidence/uc05_invariant_matrix.json` 经
     `milestones/ei1/uc05_invariant_matrix_schema.json` JSON Schema 校验(与步骤 2
     check_schemas 重叠无妨——此处是 G-EI1-5 的专门门,报告面自洽不依赖他门执行序)。
  B. **三方一致性(报告面)**:
     b1 矩阵每条 `corpus` 路径**真实存在**;`report_only` 档(I9/I10)按其口径校验(不锚 reject
        语料而锚 device/host 观测面,不得混档);
     b2 `conformance/uc05/{reject,assembly}` **磁盘实际文件集** ↔ 矩阵 `corpus` 字段**双向**互查
        ——矩阵写了不存在的语料 → 红;语料存在但矩阵漏登 → 红(**例外须在 `DOCUMENTED_UNMAPPED`
        显式登记并附理由**,新增语料默认判红,fail-closed);
     b3 `evidence/uc05_comparison_report.md` §3 逐不变量表的 I 集合与矩阵**全等**(I1~I10 无多无少),
        且逐条 `clause` / `tier`(档位中文词)与矩阵一致。
  C. **documented_historical 分级字面核(RXS-0264 redline F3 的字面面)**:
     c1 报告**顶部**须含标注 `historical counters unavailable in-repo, non-reproducible,
        no fabricated figures`;矩阵 `historical_counters` 含 `unavailable in-repo`;
     c2 **I9/I10 的 Python 对照侧条目禁含任何数值** —— 对矩阵 I9/I10 全部字符串字段与整份报告,
        逐个 `Python` 出现点切出其所在**陈述段**(至下一句读点/换行/表格单元边界),断言段内
        **零数字字面**。schema 已 by-construction 封死 number 类型字段,本门补**字面扫描**,
        封死「以字符串写杜撰数字」的剩余窗口;
     c3 I9/I10 结构面:`tier == report_only` 且 `diagnostic is null`(无 in-repo 出处的数值/诊断
        一律不得出现)。
  D. **采纳判据 bench 面(RXS-0265)**:报告 §5 表内每个 bench 数字必须**逐位等于**对应 evidence
     `results.trimmed_mean`(反杜撰:报告不得写任何未出现在 evidence 的数字);evidence 的
     `uncheckable_roots` 诚实缺口披露必须在报告中同样出现(不得只在机器档里承认、在叙事面隐去)。

**内建 red_self_test(反 YAML-only)**:取**真实**矩阵/报告文本在内存中施加四类篡改(corpus 指向
不存在文件 / Python 侧插入数字 / 删一条矩阵登记令语料失配 / 改报告表条款号),逐类断言本门判定
函数报红,并断言未篡改的真实数据判绿;任一断言不成立 → 门自身失效 → exit 1。**不改动仓库任何文件**。

**blocking(exit 1)**。用法: py -3 ci/uc05_report_check.py
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MATRIX = ROOT / "evidence" / "uc05_invariant_matrix.json"
MATRIX_SCHEMA = ROOT / "milestones" / "ei1" / "uc05_invariant_matrix_schema.json"
REPORT = ROOT / "evidence" / "uc05_comparison_report.md"
CORPUS_DIRS = ["conformance/uc05/reject", "conformance/uc05/assembly"]

ALL_INVARIANTS = [f"I{i}" for i in range(1, 11)]
REPORT_ONLY = {"I9", "I10"}
HISTORICAL_MARK = "historical counters unavailable in-repo, non-reproducible, no fabricated figures"
REPORT_HEAD_LINES = 12  # 「顶部」= 报告前 12 行(标题 + 口径块)

# 报告 §3 表「档」列中文词 → 矩阵 tier(三方一致的档位映射,裁决 1)。
TIER_WORDS = {
    "编译期": "compile_time",
    "装配期": "assembly_time",
    "lib_tested": "lib_tested",
    "report_only": "report_only",
}

# 磁盘语料存在但**不占** I 编号的文档化例外(fail-closed:新增语料不在此表即判红)。
DOCUMENTED_UNMAPPED = {
    "conformance/uc05/assembly/graph_empty.rx": (
        "空图 submit 生命周期误用(RXS-0258 生命周期条目),不单独占 I 编号;拦截见证 = "
        "rurix-rt rhi.rs `rejects_lifecycle_misuse` 库单测 + 步骤 72 device 段 EXE RED"
    ),
}

# 采纳判据 bench(RXS-0265):报告 §5 行标签 → evidence 文件。
BENCH_EVIDENCE = {
    "ei1.bench.uc05_check_cold_ms": "evidence/uc05_check_cold_20260720.json",
    "ei1.bench.uc05_check_warm_ms": "evidence/uc05_check_warm_20260720.json",
}

ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def _die(msg: str) -> None:
    print(f"[uc05_report_check] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


# ───────────────────── 纯判定层(red 自检直接喂篡改数据) ─────────────────────


def parse_report_rows(report: str) -> dict[str, dict[str, str]]:
    """解析报告 §3 逐不变量表:`| I1 | 名 | 档 | 条款 | 诊断 | 语料 | 证据级 |`。"""
    rows: dict[str, dict[str, str]] = {}
    for line in report.splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 7 or not re.fullmatch(r"I([1-9]|10)", cells[0]):
            continue
        rows[cells[0]] = {
            "name": cells[1],
            "tier_word": cells[2],
            "clause": cells[3],
            "diagnostic": cells[4],
            "corpus_cell": cells[5],
            "evidence_level": cells[6],
        }
    return rows


def check_corpus_two_way(
    invs: dict[str, dict], corpus_files: set[str]
) -> list[str]:
    """矩阵 corpus ↔ 磁盘语料集**双向**互查(纯函数;corpus_files = 相对路径集)。"""
    problems: list[str] = []
    registered: set[str] = set()
    for iid, inv in invs.items():
        path = inv.get("corpus")
        tier = inv.get("tier")
        if tier == "report_only":
            if path:
                problems.append(
                    f"{iid}: report_only 档不得锚 reject/assembly 语料(混档,现 corpus={path})"
                )
            continue
        if not path:
            problems.append(f"{iid}: tier={tier} 须锚 corpus 语料路径")
            continue
        if not (ROOT / path).is_file():
            problems.append(f"{iid}: 矩阵 corpus 指向不存在的语料 {path}")
        registered.add(path)
    for f in sorted(corpus_files - registered):
        if f in DOCUMENTED_UNMAPPED:
            continue
        problems.append(f"语料 {f} 存在于磁盘但矩阵未登记(漏登;文档化例外须入 DOCUMENTED_UNMAPPED)")
    return problems


def check_report_vs_matrix(rows: dict[str, dict[str, str]], invs: dict[str, dict]) -> list[str]:
    """报告 §3 表 ↔ 矩阵:I 集合全等 + 逐条 clause / tier 一致(纯函数)。"""
    problems: list[str] = []
    if set(rows) != set(invs):
        missing = sorted(set(invs) - set(rows), key=ALL_INVARIANTS.index)
        extra = sorted(set(rows) - set(invs))
        if missing:
            problems.append(f"对照报告 §3 表缺不变量 {missing}(报告↔矩阵不等)")
        if extra:
            problems.append(f"对照报告 §3 表多出矩阵未登记的不变量 {extra}")
    for iid, row in rows.items():
        inv = invs.get(iid)
        if inv is None:
            continue
        if row["clause"] != inv.get("clause"):
            problems.append(
                f"{iid}: 报告条款 {row['clause']} ≠ 矩阵条款 {inv.get('clause')}(三方漂移)"
            )
        want_tier = TIER_WORDS.get(row["tier_word"])
        if want_tier is None:
            problems.append(f"{iid}: 报告档位词 {row['tier_word']!r} 不在裁决 1 三档映射内")
        elif want_tier != inv.get("tier"):
            problems.append(
                f"{iid}: 报告档位 {row['tier_word']}({want_tier}) ≠ 矩阵 tier {inv.get('tier')}"
            )
    return problems


# 陈述段终止符:句读 / 换行 / 表格单元边界 / 括号闭合。**不含开括号** —— 括号内小句仍留在段内,
# 否则 `Python 侧漏报(<数字>次)` 可从段外逃逸。
_SEG_STOP = re.compile(r"[。;;\n|)）]")

# **in-repo 出处标识符**白名单:段内数字**只许**以这些形态出现(条款号 / 错误码 / 不变量号 /
# 里程碑与验收门号 / RFC 号 / 章节号 / 版号),它们各有仓库内事实源、可机核。凡剥离后仍剩数字
# 字面 → 即「无 in-repo 出处的数值」→ 红(RXS-0264 redline F3 的字面面)。
_IN_REPO_IDS = re.compile(
    r"RXS-\d{4}|RFC-\d{4}|RX\d{4}|RD-\d{3}|SG-\d{3}|D-\d{3}|U\d+"
    r"|G-[A-Z]+\d*-\d+|EI1(\.\d+)?|MS1(\.\d+)?|MB1|EA1(\.\d+)?|G\d(\.\d+)?"
    r"|I(?:10|[1-9])|§\d+(\.\d+)*|v\d+(\.\d+)*"
)


def python_side_segments(text: str) -> list[str]:
    """切出每个 `Python` 出现点所在的陈述段(至下一句读点/换行/单元边界)。"""
    segs: list[str] = []
    for m in re.finditer(r"Python", text):
        rest = text[m.start():]
        stop = _SEG_STOP.search(rest, 1)
        segs.append(rest[: stop.start()] if stop else rest)
    return segs


def check_no_digits_in_python_side(label: str, text: str) -> list[str]:
    """documented_historical 字面核:Python 对照侧陈述段内**零无出处数字**(RXS-0264 F3)。

    剥离 in-repo 出处标识符(`_IN_REPO_IDS`)后仍出现的任何数字字面即判红。
    """
    problems: list[str] = []
    for seg in python_side_segments(text):
        if re.search(r"\d", _IN_REPO_IDS.sub("", seg)):
            problems.append(
                f"{label}: Python 对照侧陈述段出现无 in-repo 出处的数字字面"
                f"(杜撰数字窗口,RXS-0264 F3):{seg.strip()!r}"
            )
    return problems


def check_report_bench_numbers(report: str, bench_docs: dict[str, dict]) -> list[str]:
    """报告 §5 bench 数字 ↔ evidence results.trimmed_mean 逐位相等(反杜撰,RXS-0265)。"""
    problems: list[str] = []
    for entry_id, doc in bench_docs.items():
        want = doc["results"]["trimmed_mean"]
        row = None
        for line in report.splitlines():
            if line.startswith("|") and entry_id in line:
                row = line
                break
        if row is None:
            problems.append(f"对照报告 §5 缺 bench 行 {entry_id}(采纳判据未入叙事面)")
            continue
        nums = [float(x) for x in re.findall(r"\d+\.\d+", row)]
        if want not in nums:
            problems.append(
                f"{entry_id}: 报告 §5 数字 {nums} 不含 evidence trimmed_mean {want}"
                "(报告数字须逐位来自 evidence,禁杜撰)"
            )
    return problems


def red_self_test(matrix_text: str, report: str, corpus_files: set[str]) -> None:
    """反 YAML-only:对**真实**数据施加四类篡改须逐类判红,未篡改须判绿(不改仓库文件)。"""
    invs = {i["id"]: i for i in json.loads(matrix_text)["invariants"]}
    rows = parse_report_rows(report)

    # 绿基线:真实数据须全绿(过严即门失效)。
    for name, got in (
        ("corpus 双向互查", check_corpus_two_way(invs, corpus_files)),
        ("报告↔矩阵", check_report_vs_matrix(rows, invs)),
        ("矩阵 Python 侧字面", check_no_digits_in_python_side("matrix", matrix_text)),
        ("报告 Python 侧字面", check_no_digits_in_python_side("report", report)),
    ):
        if got:
            _die(
                f"在库数据未过判定「{name}」:{got}"
                "(自检绿基线不成立 —— 系真实漂移则修数据,系门过严则修门)"
            )

    # RED 1:某条 corpus 指向不存在的语料。
    t1 = json.loads(matrix_text)
    for inv in t1["invariants"]:
        if inv.get("corpus"):
            inv["corpus"] = "conformance/uc05/reject/__does_not_exist__.rx"
            break
    if not check_corpus_two_way({i["id"]: i for i in t1["invariants"]}, corpus_files):
        _die("red 自检失败:corpus 指向不存在语料未判红")

    # RED 2:Python 对照侧插入数字。
    t2 = matrix_text.replace("Python 侧 = 无数字定性历史陈述", "Python 侧漏报 37 次", 1)
    if t2 == matrix_text:
        _die("red 自检失败:未找到 Python 对照侧锚点(矩阵措辞漂移,自检失效)")
    if not check_no_digits_in_python_side("matrix", t2):
        _die("red 自检失败:Python 对照侧杜撰数字未判红")

    # RED 3:矩阵删一条登记 → 磁盘语料漏登。
    t3 = json.loads(matrix_text)
    t3["invariants"] = [i for i in t3["invariants"] if i["id"] != "I1"]
    if not check_corpus_two_way({i["id"]: i for i in t3["invariants"]}, corpus_files):
        _die("red 自检失败:磁盘语料漏登矩阵未判红")

    # RED 4:报告表条款号漂移。
    t4 = dict(rows)
    any_id = next(iter(t4))
    t4[any_id] = {**t4[any_id], "clause": "RXS-9999"}
    if not check_report_vs_matrix(t4, invs):
        _die("red 自检失败:报告↔矩阵条款漂移未判红")

    print("[uc05_report_check] red_self_test PASS(4 类真实篡改逐类判红 + 真实数据判绿)")


# ───────────────────────────────── 主门 ─────────────────────────────────


def main() -> int:
    for p in (MATRIX, MATRIX_SCHEMA, REPORT):
        if not p.is_file():
            _die(f"缺 {p.relative_to(ROOT)}")
    matrix_text = MATRIX.read_text(encoding="utf-8")
    matrix = json.loads(matrix_text)
    report = REPORT.read_text(encoding="utf-8")
    invs = {i["id"]: i for i in matrix.get("invariants", [])}
    rows = parse_report_rows(report)

    corpus_files: set[str] = set()
    for d in CORPUS_DIRS:
        if not (ROOT / d).is_dir():
            _die(f"缺语料目录 {d}")
        corpus_files |= {
            f"{d}/{p.name}" for p in sorted((ROOT / d).glob("*.rx"))
        }

    red_self_test(matrix_text, report, corpus_files)

    # A) schema 门。
    try:
        import jsonschema
    except ImportError:
        _die("缺 jsonschema 依赖(pip install -r requirements.txt)")
    validator = jsonschema.Draft7Validator(json.loads(MATRIX_SCHEMA.read_text(encoding="utf-8")))
    for v in validator.iter_errors(matrix):
        err(f"矩阵 schema 违例: {'/'.join(str(x) for x in v.path)}: {v.message}")

    # B) 三方一致性(报告面)。
    for want in ALL_INVARIANTS:
        if want not in invs:
            err(f"矩阵缺不变量 {want}")
    ERRORS.extend(check_corpus_two_way(invs, corpus_files))
    ERRORS.extend(check_report_vs_matrix(rows, invs))

    # C) documented_historical 字面核。
    head = "\n".join(report.splitlines()[:REPORT_HEAD_LINES])
    if HISTORICAL_MARK not in head:
        err(f"对照报告前 {REPORT_HEAD_LINES} 行缺 historical counters 顶部标注(RXS-0264)")
    if "unavailable in-repo" not in str(matrix.get("historical_counters", "")):
        err("矩阵 historical_counters 缺 `unavailable in-repo` 口径声明(RXS-0264)")
    for iid in sorted(REPORT_ONLY):
        inv = invs.get(iid)
        if inv is None:
            continue
        if inv.get("tier") != "report_only":
            err(f"{iid}: tier 应为 report_only(documented_historical)")
        if inv.get("diagnostic") is not None:
            err(f"{iid}: report_only 项不得有诊断码(diagnostic 须为 null)")
        for k, v in inv.items():
            if isinstance(v, (int, float)) and not isinstance(v, bool):
                err(f"{iid}.{k}: report_only 条目出现数值字段(无 in-repo 出处,RXS-0264 F3)")
        ERRORS.extend(
            check_no_digits_in_python_side(f"矩阵 {iid}", json.dumps(inv, ensure_ascii=False))
        )
    ERRORS.extend(check_no_digits_in_python_side("对照报告", report))

    # D) 采纳判据 bench 面(RXS-0265):报告数字 ↔ evidence 逐位相等 + 诚实缺口同步披露。
    bench_docs: dict[str, dict] = {}
    for entry_id, rel in BENCH_EVIDENCE.items():
        p = ROOT / rel
        if not p.is_file():
            err(f"{entry_id}: 缺 bench evidence {rel}")
            continue
        doc = json.loads(p.read_text(encoding="utf-8"))
        if doc.get("bench", {}).get("budget_entry") != entry_id:
            err(f"{entry_id}: evidence budget_entry 与预期条目不符({rel})")
            continue
        bench_docs[entry_id] = doc
    ERRORS.extend(check_report_bench_numbers(report, bench_docs))
    for entry_id, doc in bench_docs.items():
        for u in doc.get("uncheckable_roots", []):
            if u["file"] not in report:
                err(
                    f"{entry_id}: evidence 披露的不可 check 成员 {u['file']} 未在对照报告出现"
                    "(机器档承认而叙事面隐去,RXS-0265 诚实缺口纪律)"
                )

    if ERRORS:
        print("[uc05_report_check] FAIL")
        for e in ERRORS:
            print(f"  - {e}")
        return 1

    print(
        f"[uc05_report_check] PASS 矩阵 schema + 三方一致性(矩阵 {len(invs)} 条 ↔ 语料"
        f" {len(corpus_files)} 件〔文档化例外 {len(DOCUMENTED_UNMAPPED)}〕↔ 报告 §3 表"
        f" {len(rows)} 行)+ documented_historical 字面核(I9/I10 Python 侧零数字)+ RXS-0265"
        f" 采纳判据 bench 报告↔evidence 逐位一致({len(bench_docs)} 口径)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
