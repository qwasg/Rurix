#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C2 渲染器文档与示例）
"""G31+ 波 C Task C2：渲染器文档与示例门冒烟（g31.waveC.docs；
G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #49「渲染器文档与示例：集成指南、
pass/特性矩阵、性能调优指南、最小示例工程——新用户按文档 <1 天完成最小集成」
兑现面）。

判据闭集（milestones/g31/g31_renderer_docs_evidence_schema.json 描述段逐字）：
1. docs_present_with_anchors：docs/renderer/ 三文档（integration_guide /
   feature_matrix / performance_tuning）在树 + 关键节锚（标题逐字）全在场。
2. on_record_numbers_present：各文档关键在案 measured 数字逐字在场（防文档
   腐化——数字面漂移即红；全来自 milestones/契约与 evidence 在案值，禁新造）。
3. example_sources_present：examples/minimal_host/ 三件（minimal_host.cpp /
   build.ps1 / README.md）在树 + 关键符号/标记逐字在场。
4. walkthrough_record_valid：milestones/g31/g31_renderer_docs_walkthrough.json
   在案——schema 自述 + 8 步全 exit=0 + doc_fixes_applied ≥ 1 + ISO 时间戳形态。
5. example_build_emit_dll：rurixc apps/uc05-rhi/src/embed.rx --emit=dll →
   rurix_rhi.dll/.lib/.h 三件产出 + 生成头声明集 == 期望导出集（4 符号）。
6. example_compile_and_run_real：cl.exe 编译链接 minimal_host.cpp → 真跑
   rc=0 + 输出标记 RURIX_MINIMAL_HOST_OK passes=2 frames=4 pixel=0x00000000
   逐字（真 GPU 真跑；RXS-0277 Q-PixelCriterion 清色不变量）。
7. frozen_docs_untouched：00~13 号根冻结规划文档 git status 面零改动
   （任务纪律「不触碰 00-14 冻结文档」机器核对）。

三态：缺 rurixc/clang/MSVC/SDK 或真跑面无 Vulkan/GPU → DEV_ENV_DEGRADE 退 0
（不冒充 PASS）；RURIX_REQUIRE_REAL=1 下降级翻硬 FAIL。facts 1~4/7 = host 恒
跑面（文档损坏即硬 FAIL，不归 dev-env 降级）。

evidence 纪律：PASS 才落 evidence/g31_renderer_docs_<ts>.json（check_schemas
前缀路由 g31_renderer_docs_）；FAIL 诊断件落 .tmp/g31_gates/renderer_docs/
工作区不污染 evidence/ 路由面（fail-closed：evidence/ 无件 = 门未过）。

用法：
  py -3 ci/g31_renderer_docs_smoke.py --selftest
  py -3 ci/g31_renderer_docs_smoke.py --gate g31.waveC.docs
"""
from __future__ import annotations

import argparse
import datetime as _dt
import io
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

GATE_KEY = "g31.waveC.docs"
SUBJECT = "g31_renderer_docs"
WAVE = "G31+.C"
TAG = "g31_renderer_docs"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_renderer_docs_evidence_schema.json"
SCHEMA_ID = "rurix.g31.renderer_docs_evidence.v1"
WALKTHROUGH_PATH = ROOT / "milestones" / "g31" / "g31_renderer_docs_walkthrough.json"
EXAMPLE_DIR = ROOT / "docs" / "renderer" / "examples" / "minimal_host"
WORK = ROOT / ".tmp" / "g31_gates" / "renderer_docs"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
RURIXC = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
EMBED_RX = ROOT / "apps" / "uc05-rhi" / "src" / "embed.rx"

RUN_MARKER = "RURIX_MINIMAL_HOST_OK passes=2 frames=4 pixel=0x00000000"
EXPECTED_EXPORTS = {
    "uc05_run_graph", "uc05_graph_pass_count",
    "uc05_gfx_run_frame", "uc05_gfx_pass_count",
}

# 工具链 pin（与 ci/uc05_engine_embed_smoke.py 同源；RURIXC_CLANG 覆写 clang）。
CLANG_PIN = Path(r"C:/Program Files/LLVM/bin/clang.exe")
MSVC_ROOT_PIN = Path(r"C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.44.35207")
SDK_INC_PIN = Path(r"C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0")

# 文档判据面：关键节锚（标题逐字）+ 关键在案 measured 数字（防腐化，全部引自
# milestones/ 契约与 evidence 在案值——禁新造；改文档改数字 = 门红）。
DOC_SPECS = {
    "integration_guide": {
        "path": ROOT / "docs" / "renderer" / "integration_guide.md",
        "headings": [
            "# Rurix 渲染器集成指南",
            "## 1. 渲染器形态总览",
            "## 2. 系统要求",
            "## 3. 获取与构建",
            "## 4. 最小集成：C ABI 宿主五步",
            "## 5. 形态 B：真窗口呈现",
            "## 6. 形态 C：离屏 bench / 确定性 digest",
            "## 7. 确定性协议（固定 seed / digest 口径）",
            "## 8. 双帧率口径与三态纪律",
            "## 9. 已知缺口（诚实登记）",
        ],
        "numbers": [
            "145.30", "rurix_rhi.lib", "--emit=dll", "RXS-0252", "RXS-0253",
            "skipped_dev_env", "RURIX_REQUIRE_REAL",
        ],
    },
    "feature_matrix": {
        "path": ROOT / "docs" / "renderer" / "feature_matrix.md",
        "headings": [
            "# Rurix 渲染器 pass / 特性矩阵",
            "## 1. 生产五 pass 结构（真窗口车道）",
            "## 2. 超分三臂（`--backend` 闭集）",
            "## 3. 帧生成 FG（`--fg`，G26 kernel 生产接线）",
            "## 4. 内容面特性（G32 波 B 生产接线五大件 + 动态场景）",
            "## 5. GI 档（`--gi`，默认 off 决策在案）",
            "## 6. 组合互斥表（波 B 在案闭集，fail-closed exit=1 逐字拒跑）",
            "## 7. 状态词表",
        ],
        "numbers": [
            "2.29", "2.79", "4.01", "145.30", "2.997", "15.8", "3.68e-8",
            "1.75e-9", "+6.29%", "+4.41ms", "3.64", "0.960479", "0.956162",
            "tsr_device", "dlss_sr", "fsr_3_1_5", "maintain_default_off",
            "not_triggered",
        ],
    },
    "performance_tuning": {
        "path": ROOT / "docs" / "renderer" / "performance_tuning.md",
        "headings": [
            "# Rurix 渲染器性能调优指南",
            "## 1. 口径纪律（勿混）",
            "## 2. 基线数字（在案 measured）",
            "## 3. 调优杠杆（按收益/代价排序）",
            "## 4. 真窗口车道口径解读",
            "## 5. 已知诚实红面（知情决策）",
            "## 6. 自助测量清单",
        ],
        "numbers": [
            "2.29", "−17.2%", "−16.2%", "−23.5%", "145.30", "0.960479",
            "0.956162", "0.980232", "3.64", "+4.41ms", "+6.29%", "1.8344",
            "1.5185",
        ],
    },
}

EXAMPLE_SPECS = {
    "minimal_host.cpp": [
        "uc05_gfx_pass_count", "uc05_gfx_run_frame", "RURIX_MINIMAL_HOST_OK",
        "rurix_rhi.h", "步骤 1", "步骤 5",
    ],
    "build.ps1": [
        "--emit=dll", "rurix_rhi.lib", "RURIX_MINIMAL_HOST_OK", "DEV_ENV_DEGRADE",
    ],
    "README.md": [
        "build.ps1", "RURIX_MINIMAL_HOST_OK", "pixel=0x00000000",
    ],
}

FROZEN_DOCS = [
    "00_MASTER_INDEX.md", "01_VISION_AND_MISSION.md", "02_USERS_AND_USE_CASES.md",
    "04_DESIGN_PRINCIPLES.md", "08_RUNTIME_AND_TOOLING.md", "09_STDLIB_AND_ECOSYSTEM.md",
    "10_GOVERNANCE.md", "11_ROADMAP.md", "12_RISKS.md", "13_DECISION_LOG.md",
]

FACT_IDS = [
    "docs_present_with_anchors",
    "on_record_numbers_present",
    "example_sources_present",
    "walkthrough_record_valid",
    "example_build_emit_dll",
    "example_compile_and_run_real",
    "frozen_docs_untouched",
]

ISO_TS_RE = re.compile(r"^20\d\d-\d\d-\d\dT\d\d:\d\d:\d\dZ$")
FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 3600, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


# ---------------------------------------------------------------------------
# 判读器（selftest 红绿两臂消费面；全纯函数无 GPU/工具链依赖）
# ---------------------------------------------------------------------------


def missing_tokens(text: str, tokens: list[str]) -> list[str]:
    """逐字在场核验（标题/数字/符号同一判据面）——缺项清单（空 = 全在场）。"""
    return [t for t in tokens if t not in text]


def walkthrough_problems(doc: dict) -> list[str]:
    """走通记录核验：schema 自述 / 8 步全 exit=0（各步带 n/exit/wall_s）/
    doc_fixes_applied ≥ 1 / walkthrough_at_utc ISO 形态。"""
    problems = []
    if doc.get("schema") != "rurix.g31.renderer_docs_walkthrough.v1":
        problems.append(f"schema 自述异常: {doc.get('schema')!r}")
    steps = doc.get("steps")
    if not isinstance(steps, list) or len(steps) != 8:
        problems.append(f"steps 非 8 项: {type(steps).__name__} len={len(steps) if isinstance(steps, list) else '-'}")
        steps = steps if isinstance(steps, list) else []
    for i, s in enumerate(steps, 1):
        if not isinstance(s, dict) or s.get("n") != i or s.get("exit") != 0 or not isinstance(s.get("wall_s"), (int, float)):
            problems.append(f"step {i} 形态/退出码异常: {str(s)[:80]}")
    summary = doc.get("summary") or {}
    if not isinstance(summary.get("doc_fixes_applied"), int) or summary["doc_fixes_applied"] < 1:
        problems.append("summary.doc_fixes_applied < 1（卡点修正面缺失）")
    ts = doc.get("walkthrough_at_utc")
    if not isinstance(ts, str) or ISO_TS_RE.match(ts) is None:
        problems.append(f"walkthrough_at_utc 非 ISO UTC 形态: {ts!r}")
    return problems


def frozen_violations(porcelain_text: str) -> list[str]:
    """git status --porcelain 文本 → 冻结根规划文档改动清单（空 = 零触碰）。
    非 ?? 状态（M/A/D/R 等）命中冻结清单即违例。"""
    out = []
    for line in porcelain_text.splitlines():
        if len(line) < 4:
            continue
        status, path = line[:2], line[3:].strip().strip('"')
        name = path.rsplit("/", 1)[-1]
        if "/" not in path and name in FROZEN_DOCS and status != "??":
            out.append(f"{status} {path}")
    return out


def header_exports(header_text: str) -> set[str]:
    """生成头声明集（同 ci/uc05_engine_embed_smoke.py header_names 口径）。"""
    names: set[str] = set()
    for line in header_text.splitlines():
        s = line.strip()
        if s.endswith(";") and "(" in s and not s.startswith(("#", "/", "extern", "}")):
            m = re.search(r"(\w+)\s*\(", s)
            if m:
                names.add(m.group(1))
    return names


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决：无降级 → None（续跑）；有降级 + REQUIRE_REAL → 1（硬红）；
    有降级无 REQUIRE_REAL → 0（SKIP 非 PASS 非 FAIL）。"""
    if not degrade:
        return None
    return 1 if require_real else 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def resolve_clang() -> Path | None:
    v = os.environ.get("RURIXC_CLANG")
    if v and Path(v).is_file():
        return Path(v)
    if CLANG_PIN.is_file():
        return CLANG_PIN
    from shutil import which

    w = which("clang")
    return Path(w) if w else None


def resolve_msvc() -> tuple[Path, Path, Path] | None:
    """→ (cl.exe, msvc_root, sdk_inc_root)；pin 优先，版本目录回退最新。"""
    candidates = [MSVC_ROOT_PIN]
    vs_root = Path(r"C:/Program Files/Microsoft Visual Studio/2022")
    if vs_root.is_dir():
        for msvc in sorted(vs_root.glob("*/VC/Tools/MSVC/*"), reverse=True):
            candidates.append(msvc)
    sdk_inc = None
    kits_inc = Path(r"C:/Program Files (x86)/Windows Kits/10/Include")
    if SDK_INC_PIN.is_dir():
        sdk_inc = SDK_INC_PIN
    elif kits_inc.is_dir():
        vers = sorted((p for p in kits_inc.iterdir() if p.is_dir()), reverse=True)
        sdk_inc = vers[0] if vers else None
    for msvc_root in candidates:
        cl = msvc_root / "bin" / "Hostx64" / "x64" / "cl.exe"
        if cl.is_file() and (msvc_root / "include").is_dir() and sdk_inc is not None:
            return cl, msvc_root, sdk_inc
    return None


def run_gate() -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1

    # ── fact 1/2：三文档在树 + 节锚 + 在案数字（host 恒跑面）──
    docs_info: dict[str, dict] = {}
    for key, spec in DOC_SPECS.items():
        p: Path = spec["path"]
        if not p.is_file():
            docs_info[key] = {
                "path": str(p.relative_to(ROOT)).replace("\\", "/"), "bytes": 0,
                "headings_present": 0, "headings_required": len(spec["headings"]),
                "numbers_present": 0, "numbers_required": len(spec["numbers"]),
                "_missing_headings": spec["headings"], "_missing_numbers": spec["numbers"],
            }
            continue
        text = p.read_text(encoding="utf-8")
        mh = missing_tokens(text, spec["headings"])
        mn = missing_tokens(text, spec["numbers"])
        docs_info[key] = {
            "path": str(p.relative_to(ROOT)).replace("\\", "/"),
            "bytes": p.stat().st_size,
            "headings_present": len(spec["headings"]) - len(mh),
            "headings_required": len(spec["headings"]),
            "numbers_present": len(spec["numbers"]) - len(mn),
            "numbers_required": len(spec["numbers"]),
            "_missing_headings": mh, "_missing_numbers": mn,
        }
    h_ok = all(not d["_missing_headings"] for d in docs_info.values())
    set_fact(
        "docs_present_with_anchors", h_ok,
        "; ".join(
            f"{k}={d['headings_present']}/{d['headings_required']}"
            + (f" 缺 {d['_missing_headings'][:2]}" if d["_missing_headings"] else "")
            for k, d in docs_info.items()
        ),
    )
    n_ok = all(not d["_missing_numbers"] for d in docs_info.values())
    set_fact(
        "on_record_numbers_present", n_ok,
        "; ".join(
            f"{k}={d['numbers_present']}/{d['numbers_required']}"
            + (f" 缺 {d['_missing_numbers'][:3]}" if d["_missing_numbers"] else "")
            for k, d in docs_info.items()
        ),
    )

    # ── fact 3：示例三件 + 关键符号 ──
    ex_missing: dict[str, list[str]] = {}
    for name, tokens in EXAMPLE_SPECS.items():
        p = EXAMPLE_DIR / name
        if not p.is_file():
            ex_missing[name] = ["<文件缺失>"] + tokens
            continue
        ex_missing[name] = missing_tokens(p.read_text(encoding="utf-8"), tokens)
    ex_ok = all(not v for v in ex_missing.values())
    set_fact(
        "example_sources_present", ex_ok,
        "; ".join(f"{k}{' 缺 ' + str(v[:3]) if v else ' ok'}" for k, v in ex_missing.items()),
    )

    # ── fact 4：走通记录在案 ──
    wt_problems: list[str]
    if not WALKTHROUGH_PATH.is_file():
        wt_problems = [f"走通记录缺失 {WALKTHROUGH_PATH}"]
    else:
        try:
            wt_problems = walkthrough_problems(
                json.loads(WALKTHROUGH_PATH.read_text(encoding="utf-8"))
            )
        except json.JSONDecodeError as e:
            wt_problems = [f"走通记录 JSON 解析失败: {e}"]
    set_fact(
        "walkthrough_record_valid", not wt_problems,
        "8 步全绿 + 修正面 + ISO 时间戳在案" if not wt_problems else "; ".join(wt_problems[:3]),
    )

    # ── fact 7：冻结根规划文档零触碰（git 面）──
    rp = run(["git", "status", "--porcelain"], timeout=120)
    viol = frozen_violations(rp.stdout or "") if rp.returncode == 0 else ["git status 失败"]
    set_fact(
        "frozen_docs_untouched", not viol,
        f"00~13 号根冻结规划文档 {len(FROZEN_DOCS)} 件零改动（git status --porcelain 机核）"
        if not viol else "违例: " + "; ".join(viol[:3]),
    )

    # ── host 面任一红 → 硬 FAIL（不归 dev-env 降级）──
    host_ok = h_ok and n_ok and ex_ok and not wt_problems and not viol
    if not host_ok:
        for fid in ("example_build_emit_dll", "example_compile_and_run_real"):
            set_fact(fid, False, "host 文档面前置红，构建腿未执行")
        return finalize(facts, docs_info, None, wt_problems)

    # ── dev-env 探针（构建腿前提）──
    degrade: list[str] = []
    clang = resolve_clang()
    if clang is None:
        degrade.append("clang 22.1.x 缺失（RURIXC_CLANG / LLVM pin / PATH 全未命中）")
    msvc = resolve_msvc()
    if msvc is None:
        degrade.append("MSVC cl.exe + Windows SDK 缺失")
    if not EMBED_RX.is_file():
        degrade.append(f"导出面源缺失 {EMBED_RX}")
    if not RURIXC.is_file():
        rb = run(["cargo", "build", "-q", "-p", "rurixc", "--bin", "rurixc"], timeout=3600)
        if rb.returncode != 0 or not RURIXC.is_file():
            degrade.append(f"rurixc 构建失败: {(rb.stdout + rb.stderr)[-200:]}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g31.renderer_docs.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但工具链面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP，非 PASS 非 FAIL）")
        return 0

    # ── fact 5：--emit=dll 三件 + 生成头导出集 ──
    WORK.mkdir(parents=True, exist_ok=True)
    stem = WORK / "rurix_rhi"
    env = dict(os.environ)
    env["RURIXC_CLANG"] = str(clang)
    re_emit = run([str(RURIXC), str(EMBED_RX), "--emit=dll", "-o", str(stem)], env=env)
    dll, imp_lib, hdr = stem.with_suffix(".dll"), stem.with_suffix(".lib"), stem.with_suffix(".h")
    products_ok = re_emit.returncode == 0 and dll.is_file() and imp_lib.is_file() and hdr.is_file()
    exports = header_exports(hdr.read_text(encoding="utf-8")) if hdr.is_file() else set()
    emit_ok = products_ok and exports == EXPECTED_EXPORTS
    set_fact(
        "example_build_emit_dll", emit_ok,
        f"emit rc={re_emit.returncode} 三件={'齐' if products_ok else '缺'} "
        f"生成头导出集={sorted(exports)}（期望 {sorted(EXPECTED_EXPORTS)}）",
    )

    # ── fact 6：cl 编译链接 + 真跑 ──
    run_rc, run_out = -1, ""
    if emit_ok and msvc is not None:
        cl_exe, msvc_root, sdk_inc = msvc
        sdk_lib = Path(str(sdk_inc).replace("\\Include\\", "\\Lib\\"))
        henv = dict(os.environ)
        henv["INCLUDE"] = os.pathsep.join([
            str(msvc_root / "include"), str(sdk_inc / "ucrt"), str(sdk_inc / "shared"),
            str(sdk_inc / "um"), str(WORK),
        ])
        henv["LIB"] = os.pathsep.join([
            str(msvc_root / "lib" / "x64"), str(sdk_lib / "ucrt" / "x64"),
            str(sdk_lib / "um" / "x64"), str(WORK),
        ])
        henv["PATH"] = str(msvc_root / "bin" / "Hostx64" / "x64") + os.pathsep + henv.get("PATH", "")
        host_exe = WORK / f"minimal_host{EXE_SUFFIX}"
        rc_cl = run([
            str(cl_exe), "/std:c++17", "/EHsc", "/nologo",
            str(EXAMPLE_DIR / "minimal_host.cpp"), f"/Fe:{host_exe}",
            "/link", f"/LIBPATH:{WORK}", "rurix_rhi.lib",
        ], env=henv)
        if rc_cl.returncode != 0 or not host_exe.is_file():
            set_fact("example_compile_and_run_real", False,
                     f"cl 编译链接失败 rc={rc_cl.returncode}: {(rc_cl.stdout + rc_cl.stderr)[-200:]}")
            return finalize(facts, docs_info, {
                "dir": str(EXAMPLE_DIR.relative_to(ROOT)).replace("\\", "/"),
                "emit": emit_ok, "compile_rc": rc_cl.returncode,
            }, wt_problems)
        rr = run([str(host_exe)], timeout=900)
        run_rc = rr.returncode
        run_out = (rr.stdout or "") + (rr.stderr or "")
        io.open(WORK / "minimal_host_run.log", "w", encoding="utf-8", newline="\n").write(run_out)
        # 真跑面失败归因：自检出参/标记违例 = 硬 FAIL；无 Vulkan/GPU 形态 = 降级。
        if run_rc != 0 or RUN_MARKER not in run_out:
            envish = any(k in run_out for k in ("vkCreateInstance", "Vulkan", "skipped_dev_env", "no Vulkan"))
            if envish and os.environ.get("RURIX_REQUIRE_REAL") != "1":
                note(f"DEV_ENV_DEGRADE 真跑面（无 Vulkan/GPU）: {run_out.strip()[-200:]}")
                note("SKIP DEV_ENV_DEGRADE（三态之 SKIP，非 PASS 非 FAIL）")
                return 0
            set_fact("example_compile_and_run_real", False,
                     f"真跑 rc={run_rc} 标记缺失: {run_out.strip()[-200:]}")
            return finalize(facts, docs_info, {
                "dir": str(EXAMPLE_DIR.relative_to(ROOT)).replace("\\", "/"),
                "emit": emit_ok, "compile_rc": 0, "run_rc": run_rc,
            }, wt_problems)
    run_ok = run_rc == 0 and RUN_MARKER in run_out
    set_fact(
        "example_compile_and_run_real", run_ok,
        f"cl 编译链接 rc=0 + 真跑 rc={run_rc} + 标记逐字 {RUN_MARKER!r}",
    )

    example_info = {
        "dir": str(EXAMPLE_DIR.relative_to(ROOT)).replace("\\", "/"),
        "emit": emit_ok, "compile_rc": 0, "run_rc": run_rc,
    }
    return finalize(facts, docs_info, example_info, wt_problems)


def finalize(facts: dict, docs_info: dict, example_info: dict | None, wt_problems: list[str]) -> int:
    """门裁决 + evidence 落盘（PASS → evidence/；FAIL → .tmp 工作区）。"""
    all_pass = all(f["status"] == "PASS" for f in facts.values()) and not FAILURES
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    docs_clean = {
        k: {kk: vv for kk, vv in d.items() if not kk.startswith("_")}
        for k, d in docs_info.items()
    }
    wt_steps = 0
    wt_fixes = 0
    wt_ts = "1970-01-01T00:00:00Z"
    if WALKTHROUGH_PATH.is_file():
        try:
            wt_doc = json.loads(WALKTHROUGH_PATH.read_text(encoding="utf-8"))
            wt_steps = len(wt_doc.get("steps") or [])
            wt_fixes = int((wt_doc.get("summary") or {}).get("doc_fixes_applied") or 0)
            wt_ts = str(wt_doc.get("walkthrough_at_utc") or wt_ts)
        except (json.JSONDecodeError, ValueError):
            pass
    env_info = {
        "os": "windows",
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    example_doc = {
        "dir": (example_info or {}).get("dir", "docs/renderer/examples/minimal_host"),
        "sources": sorted(EXAMPLE_SPECS.keys()),
        "emit_products": {
            "dll": ".tmp/g31_gates/renderer_docs/rurix_rhi.dll",
            "import_lib": ".tmp/g31_gates/renderer_docs/rurix_rhi.lib",
            "header": ".tmp/g31_gates/renderer_docs/rurix_rhi.h",
        },
        "header_exports": sorted(EXPECTED_EXPORTS),
        "run_marker": RUN_MARKER,
        "run_rc": (example_info or {}).get("run_rc", 0 if all_pass else -1),
    }
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": [facts[fid] for fid in FACT_IDS],
        "docs": docs_clean,
        "example": example_doc,
        "walkthrough": {
            "path": "milestones/g31/g31_renderer_docs_walkthrough.json",
            "steps": wt_steps,
            "doc_fixes_applied": wt_fixes,
            "timestamp_iso": wt_ts,
        },
        "frozen_docs": {
            "checked": len(FROZEN_DOCS),
            "violations": 0 if facts["frozen_docs_untouched"]["status"] == "PASS" else 1,
            "method": "git status --porcelain 非 ?? 状态命中冻结清单即违例（00~13 号根规划文档）",
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C2 渲染器文档与示例门（G31_PLUS §5 #49 兑现面）：三文档节锚 + 在案 "
            "measured 数字逐字防腐化 + 示例三件符号面 + walkthrough 记录在案 + rurixc --emit=dll "
            "三件 + 生成头导出集 4 符号 + cl 编译链接真跑标记逐字（RXS-0277 清色不变量）+ 冻结 "
            "根规划文档零触碰。三态：缺工具链/GPU → DEV_ENV_DEGRADE SKIP 退 0 不冒充；"
            "RURIX_REQUIRE_REAL=1 翻硬 FAIL。facts: "
            + "; ".join(f["id"] + "=" + f["status"] for f in (facts[fid] for fid in FACT_IDS))
        ),
    }
    if all_pass:
        import jsonschema  # 自校验硬门（schema 漂移即 RED）

        errs = list(jsonschema.Draft7Validator(
            json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        ).iter_errors(gate_doc))
        if errs:
            for e in errs[:5]:
                fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
            all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_renderer_docs_{ts}.json"
    else:
        WORK.mkdir(parents=True, exist_ok=True)
        gate_path = WORK / f"gate_fail_{ts}.json"
    io.open(gate_path, "w", encoding="utf-8", newline="\n").write(
        json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n"
    )
    note(f"evidence: {gate_path.relative_to(ROOT)}")
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂，无 GPU/工具链依赖）
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # 红绿臂①：逐字在场核验。
    expect(missing_tokens("abc 145.30 rurix_rhi.lib", ["145.30", "rurix_rhi.lib"]) == [],
           "GREEN:数字/符号全在场")
    expect(missing_tokens("abc", ["145.30"]) == ["145.30"], "RED:缺数字必检出")
    expect(missing_tokens("## 1. 渲染器形态总览\nx", ["## 1. 渲染器形态总览"]) == [],
           "GREEN:节锚在场")
    expect(missing_tokens("# t\n", ["## 2. 系统要求"]) == ["## 2. 系统要求"], "RED:缺节锚必检出")
    expect(missing_tokens("", []) == [], "GREEN:空需求闭集恒绿")

    # 红绿臂②：walkthrough 核验。
    good_wt = {
        "schema": "rurix.g31.renderer_docs_walkthrough.v1",
        "walkthrough_at_utc": "2026-08-26T11:14:56Z",
        "steps": [{"n": i, "exit": 0, "wall_s": 0.1} for i in range(1, 9)],
        "summary": {"doc_fixes_applied": 4},
    }
    expect(walkthrough_problems(good_wt) == [], "GREEN:合法走通记录")
    bad_schema = dict(good_wt, schema="rurix.other.v1")
    expect(any("schema" in p for p in walkthrough_problems(bad_schema)), "RED:schema 异必检出")
    bad_steps = dict(good_wt, steps=good_wt["steps"][:7])
    expect(any("steps" in p for p in walkthrough_problems(bad_steps)), "RED:7 步必检出")
    bad_exit = dict(good_wt, steps=[dict(s, exit=1) if s["n"] == 3 else s for s in good_wt["steps"]])
    expect(any("step 3" in p for p in walkthrough_problems(bad_exit)), "RED:步 exit=1 必检出")
    bad_fix = dict(good_wt, summary={"doc_fixes_applied": 0})
    expect(any("doc_fixes_applied" in p for p in walkthrough_problems(bad_fix)), "RED:零修正必检出")
    bad_ts = dict(good_wt, walkthrough_at_utc="2026-08-26 11:14")
    expect(any("ISO" in p for p in walkthrough_problems(bad_ts)), "RED:非 ISO 时间戳必检出")
    expect(walkthrough_problems({}) != [], "RED:空文档必检出")

    # 红绿臂③：冻结面核验。
    expect(frozen_violations(" M src/foo.rs\n?? docs/\n") == [], "GREEN:非冻结面改动不违例")
    expect(frozen_violations(" M 11_ROADMAP.md\n") == [" M 11_ROADMAP.md"], "RED:冻结文档改动必检出")
    expect(frozen_violations("M  13_DECISION_LOG.md") == ["M  13_DECISION_LOG.md"],
           "RED:staged 改动必检出")
    expect(frozen_violations("?? 11_ROADMAP.md") == [], "GREEN:?? 不计改动（冻结件均 tracked）")
    expect(frozen_violations(" M docs/00_MASTER_INDEX.md/x") == [], "GREEN:子路径同名不误判")

    # 红绿臂④：生成头声明集解析。
    hdr_text = (
        "/* Generated by rurixc --emit=dll (RXS-0253). Do not edit. */\n"
        "#ifndef X\n#define X\n#include <stdint.h>\n#ifdef __cplusplus\nextern \"C\" {\n#endif\n"
        "int32_t uc05_run_graph(int32_t*, int32_t);\n"
        "int32_t uc05_graph_pass_count(void);\n"
        "int32_t uc05_gfx_run_frame(uint32_t*, int32_t, int32_t);\n"
        "int32_t uc05_gfx_pass_count(void);\n"
        "#ifdef __cplusplus\n}\n#endif\n#endif\n"
    )
    expect(header_exports(hdr_text) == EXPECTED_EXPORTS, "GREEN:四符号声明集解析")
    expect(header_exports(hdr_text.replace("uc05_gfx_pass_count(void);\n", "", 1)) != EXPECTED_EXPORTS,
           "RED:缺符号必检出")
    expect(header_exports("int32_t uc05_extra(void);\n") != EXPECTED_EXPORTS, "RED:多符号必检出")

    # 红绿臂⑤：三态裁决。
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")

    # 红绿臂⑥：事实源互核（真文档/示例/记录/schema 在树 + 判据面自洽）。
    for key, spec in DOC_SPECS.items():
        p: Path = spec["path"]
        expect(p.is_file(), f"文档在树 {key}")
        if p.is_file():
            text = p.read_text(encoding="utf-8")
            expect(missing_tokens(text, spec["headings"]) == [], f"节锚全在场 {key}")
            expect(missing_tokens(text, spec["numbers"]) == [], f"在案数字全在场 {key}")
    for name in EXAMPLE_SPECS:
        expect((EXAMPLE_DIR / name).is_file(), f"示例件在树 {name}")
    expect(WALKTHROUGH_PATH.is_file(), "走通记录在树")
    if WALKTHROUGH_PATH.is_file():
        expect(walkthrough_problems(json.loads(WALKTHROUGH_PATH.read_text(encoding="utf-8"))) == [],
               "走通记录判据面全绿")
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "facts", "docs", "example",
                "walkthrough", "frozen_docs", "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核（12 字段）",
        )
        fact_enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(fact_enum) == sorted(FACT_IDS), "facts id 枚举闭集互核（7 facts）")
        expect(gs["properties"]["example"]["properties"]["run_marker"]["const"] == RUN_MARKER,
               "run_marker const 互核")
        expect(gs["properties"]["walkthrough"]["properties"]["steps"]["const"] == 8,
               "walkthrough steps const 互核")
    expect(len(FACT_IDS) == 7, "facts 闭集 = 7")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=7；6 红臂组 + 事实源互核 + schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
