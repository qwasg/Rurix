#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""文档站 `rx doc` 生成冒烟(M8 CI_GATES §2 步骤 39,契约 G-M8-6 子项;CPU-only,check_ 守卫风格)。

`rx doc` 从既有单一事实源(spec/*.md + registry/error_codes.json + conformance/traceability_matrix.json)
确定性生成静态文档站。本门断言:
  绿:`cargo build -p rx` → `rx doc --out <A>` 与 `--out <B>` 两次生成,4 关键页(index/spec/errors/
    traceability)**逐字节一致**(确定性可复现);且**关键页齐备 + 锚点齐备**——spec.html 含独立扫描
    spec/*.md 得到的**每条** RXS 条款锚点(`id="RXS-####"`),errors.html 含 error_codes.json 的**每个**
    错误码索引项(`id="RX####"`)。
  红(反 YAML-only,CI_GATES §6):站点产物缺关键页 / 缺一条款锚点 / 缺错误码索引项 → 结构核验判红。
内置 red 自检:对合成的「缺页 / 缺锚点」伪站点断言核验器判红、对「齐备」伪站点断言判绿——证明门真在
比对结构而非空过。

全绿 → 写 evidence/doc_site_smoke.json(generation_complete=true + 确定性 + 锚点计数 + facts + redgreen)
+ 退出 0。门为 check_* 守卫风格,**不写 budget counter**(CI_GATES §2.39);失败即红(非零退出)。
"""
import datetime
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RX = ROOT / "target" / "debug" / ("rx.exe" if sys.platform == "win32" else "rx")
OUT_DIR = ROOT / "build" / "doc_site_smoke"
EVIDENCE = ROOT / "evidence" / "doc_site_smoke.json"
PAGES = ("index.html", "spec.html", "errors.html", "traceability.html")
RUN_URL_TODO = "TODO:回填 self-hosted runner 绿→缺关键页/缺锚点(红)→复原(绿)run URL(步骤 39,CI_GATES §6)"


def fail(msg: str) -> None:
    print(f"[doc_site_smoke] FAIL: {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


# ── 独立事实源扫描(与 rx doc 实现彼此独立,构成交叉核对)──

def scan_spec_clause_ids() -> set:
    """独立扫描 spec/*.md 的 `### RXS-####` 头(与 ci/trace_matrix.py 同口径)。"""
    ids = set()
    pat = re.compile(r"^### (RXS-\d{4})\b")
    for md in sorted((ROOT / "spec").glob("*.md")):
        for line in md.read_text(encoding="utf-8").splitlines():
            m = pat.match(line)
            if m:
                ids.add(m.group(1))
    return ids


def scan_error_code_ids() -> set:
    data = json.loads((ROOT / "registry" / "error_codes.json").read_text(encoding="utf-8"))
    return {e["id"] for e in data.get("entries", [])}


# ── 结构核验(红绿同一函数;反 YAML-only)──

def site_problems(site: Path, clause_ids: set, error_ids: set) -> list:
    """返回站点产物的缺项清单(空 = 齐备)。关键页缺失 / spec 缺条款锚点 / errors 缺错误码索引项。"""
    problems = []
    for page in PAGES:
        if not (site / page).is_file():
            problems.append(f"缺关键页 {page}")
    spec_html = site / "spec.html"
    if spec_html.is_file():
        text = spec_html.read_text(encoding="utf-8")
        missing = sorted(cid for cid in clause_ids if f'id="{cid}"' not in text)
        if missing:
            problems.append(f"spec.html 缺条款锚点 {len(missing)} 条(示例 {missing[:5]})")
    errors_html = site / "errors.html"
    if errors_html.is_file():
        text = errors_html.read_text(encoding="utf-8")
        missing = sorted(eid for eid in error_ids if f'id="{eid}"' not in text)
        if missing:
            problems.append(f"errors.html 缺错误码索引项 {len(missing)} 条(示例 {missing[:5]})")
    return problems


def page_shas(site: Path) -> list:
    """4 关键页 (文件名, SHA-256) 按文件名排序(确定性比较口径)。"""
    out = []
    for name in PAGES:
        p = site / name
        if p.is_file():
            out.append((name, hashlib.sha256(p.read_bytes()).hexdigest()))
    return out


# ── 真实生成 ──

def build_rx() -> None:
    r = run(["cargo", "build", "-p", "rx"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build -p rx 失败:\n{r.stderr}")
    if not RX.exists():
        fail(f"rx 产物缺失: {RX}")


def gen_site(out: Path) -> None:
    if out.exists():
        for p in out.glob("*.html"):
            p.unlink()
    out.mkdir(parents=True, exist_ok=True)
    r = run([str(RX), "doc", "--root", str(ROOT), "--out", str(out)])
    if r.returncode != 0:
        fail(f"rx doc 退出码 {r.returncode}:\n{r.stdout}{r.stderr}")


def red_self_test() -> None:
    """合成伪站点验证核验器:缺页/缺锚点 → 判红;齐备 → 判绿(否则门空过 → fail)。"""
    base = OUT_DIR / "_selftest"
    clause_ids = {"RXS-0001", "RXS-0002"}
    error_ids = {"RX0001"}

    bad = base / "bad"
    if bad.exists():
        for p in bad.glob("*.html"):
            p.unlink()
    bad.mkdir(parents=True, exist_ok=True)
    (bad / "index.html").write_text("x", encoding="utf-8")  # 缺 spec/errors/traceability
    (bad / "spec.html").write_text('<section id="RXS-0001">仅一条</section>', encoding="utf-8")  # 缺 RXS-0002
    if not site_problems(bad, clause_ids, error_ids):
        fail("red 自检失败:核验器未发现缺关键页/缺锚点(门空过,反 YAML-only)")

    good = base / "good"
    if good.exists():
        for p in good.glob("*.html"):
            p.unlink()
    good.mkdir(parents=True, exist_ok=True)
    (good / "index.html").write_text("ok", encoding="utf-8")
    (good / "traceability.html").write_text("ok", encoding="utf-8")
    (good / "spec.html").write_text('id="RXS-0001" id="RXS-0002"', encoding="utf-8")
    (good / "errors.html").write_text('id="RX0001"', encoding="utf-8")
    if site_problems(good, clause_ids, error_ids):
        fail("red 自检失败:齐备伪站点被误判缺项(核验器过严)")


def preserved_run_url() -> str:
    """保留已回填的 run URL,避免后续绿跑把证据刷回 TODO。"""
    if not EVIDENCE.is_file():
        return RUN_URL_TODO
    try:
        prior = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return RUN_URL_TODO
    run_url = prior.get("redgreen", {}).get("run_url")
    if isinstance(run_url, str) and run_url and not run_url.startswith("TODO:"):
        return run_url
    return RUN_URL_TODO


def main() -> None:
    red_self_test()
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    build_rx()

    clause_ids = scan_spec_clause_ids()
    error_ids = scan_error_code_ids()
    if not clause_ids:
        fail("spec/*.md 未扫到任何 RXS 条款(事实源异常)")
    if not error_ids:
        fail("registry/error_codes.json 未扫到任何错误码(事实源异常)")

    run_a = OUT_DIR / "run_a"
    run_b = OUT_DIR / "run_b"
    gen_site(run_a)
    gen_site(run_b)

    # (1) 确定性:两次生成 4 关键页逐字节一致。
    shas_a = page_shas(run_a)
    shas_b = page_shas(run_b)
    if len(shas_a) != len(PAGES):
        fail(f"关键页不齐(生成 {len(shas_a)}/{len(PAGES)}):{[n for n, _ in shas_a]}")
    deterministic = shas_a == shas_b
    if not deterministic:
        fail(f"两次生成关键页 SHA-256 不一致(非确定性):\n  A={shas_a}\n  B={shas_b}")

    # (2) 结构齐备:关键页 + 每条条款锚点 + 每个错误码索引项。
    problems = site_problems(run_a, clause_ids, error_ids)
    if problems:
        fail("站点产物缺项(反 YAML-only 红):\n  " + "\n  ".join(problems))

    doc = {
        "schema_version": 1,
        "subject": "doc_site",
        "generation_complete": True,
        "deterministic_rebuild_ok": True,
        "clause_anchor_count": len(clause_ids),
        "error_code_index_count": len(error_ids),
        "page_count": len(shas_a),
        "facts": [
            {
                "kind": "generation",
                "name": "rx_doc_pages_present",
                "note": f"4 关键页齐备({', '.join(PAGES)})",
            },
            {
                "kind": "determinism",
                "name": "byte_identical_rebuild",
                "note": "同输入两次生成 4 关键页逐字节 SHA-256 一致",
            },
            {
                "kind": "anchors",
                "name": "spec_clause_anchors_complete",
                "note": f"spec.html 含独立扫描 spec/*.md 的全部 {len(clause_ids)} 条 RXS 条款锚点",
            },
            {
                "kind": "anchors",
                "name": "error_code_index_complete",
                "note": f"errors.html 含 error_codes.json 的全部 {len(error_ids)} 个错误码索引项",
            },
        ],
        "redgreen": {
            "red_command": "删一关键页 / 抹一条款锚点 / 抹一错误码索引项 → py -3 ci/doc_site_smoke.py 退出 1",
            "red_detected": True,
            "green_command": "py -3 ci/doc_site_smoke.py",
            "green_exit_code": 0,
            "run_url": preserved_run_url(),
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[doc_site_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}"
        f"(rx doc 确定性生成 {len(PAGES)} 页 / {len(clause_ids)} 条款锚点 / {len(error_ids)} 错误码索引)"
    )
    sys.exit(0)


if __name__ == "__main__":
    main()
