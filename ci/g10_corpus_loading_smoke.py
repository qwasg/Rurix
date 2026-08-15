#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.3 波）
"""G10.3 M132 语料加载硬门冒烟（步骤 174；g10.p0.m132.corpus_loading；
RFC-0027 §4.3/§4.4；spec/external_reference.md RXS-0382/RXS-0383；
G10_ACCEPTANCE_MAP §1 M132 行判据逐字）。

host+device 性质中的 device 面 = Rurix 既有 glTF 导入面（G8 M81 rxcook
import-gltf --emit-digest 已验收面）真实加载；本门为纯 host 驱动（GPU 帧出图
归 G10.4/10.5），device_section_state 正常态 not_applicable；缓存根不可达时
dev_env_degrade 诚实登记不充绿（R-G10-9）。

判据（契约 §4.2 M132 逐字）：

  场景清单逐场景 Rurix 加载成功 + 三角形/材质/纹理计数非空 + 加载事件序列
  golden；计数为零冒充成功即 RED；静默丢场景即 RED。加载门前逐文件实算
  SHA-256 并复算清单级 canonical digest（RXS-0382 L4），不符即 fail-closed。

RED 语料（milestones/g10/red_fixtures/m132/）：
  red_zero_count_masquerade.json   计数为零冒充成功
  red_silent_scene_drop.json       静默丢场景（loaded 集缺 ready 场景）
  red_all_not_ready_manifest.json  全 not-ready 空清单 vacuous 冒充

用法：
  py -3 ci/g10_corpus_loading_smoke.py --gate g10.p0.m132.corpus_loading
  py -3 ci/g10_corpus_loading_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m132_corpus_loading_evidence_schema.json"
GOLDEN_PATH = ROOT / "milestones" / "g10" / "g10_corpus_loading_golden.json"
RED_FIXTURE_DIR = ROOT / "milestones" / "g10" / "red_fixtures" / "m132"

sys.path.insert(0, str(ROOT / "ci"))
import g10_corpus_lib as lib  # noqa: E402

GATE_KEY = "g10.p0.m132.corpus_loading"
NUMERIC_STEP = 174
SOURCE_REF = "RFC-0027 §4.3/§4.4;spec/external_reference.md RXS-0382/RXS-0383;G10_ACCEPTANCE_MAP §1 M132"
TAG = "g10_m132"
SUBJECT = "g10_m132_corpus_loading"
MATRIX_ROW = "M132"

TABLES = ("scenes", "nodes", "meshes", "primitives", "materials", "textures")
CHECK_KEYS = [
    "manifest_valid",
    "cache_root_resolved",
    "per_scene_digest_verified",
    "per_scene_import_ok",
    "counts_nonzero",
    "counts_equal_golden",
    "event_sequence_golden",
    "scene_set_reconciled",
    "red_fixtures_all_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cmd(cmd: list[str], timeout: int = 3600) -> tuple[int, str]:
    print(f"[{TAG}] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(cmd), "exit_code": r.returncode})
    return r.returncode, r.stdout + r.stderr


def build_rxcook() -> Path | None:
    rc, blob = run_cmd(["cargo", "build", "-p", "rurix-asset", "--bin", "rxcook", "--quiet"])
    if rc != 0:
        check(False, f"rxcook 构建失败:\n{blob[-1500:]}")
        return None
    exe = ROOT / "target" / "debug" / ("rxcook.exe" if sys.platform == "win32" else "rxcook")
    if not exe.is_file():
        check(False, f"rxcook 产物缺失: {exe}")
        return None
    return exe


def extract_tables(stdout: str) -> dict | None:
    lines: list[str] = []
    for line in stdout.splitlines():
        if line.startswith("status=") or line.startswith("coverage_"):
            break
        lines.append(line)
    blob = "\n".join(lines).strip()
    try:
        return json.loads(blob) if blob else None
    except json.JSONDecodeError:
        return None


def gltf_triangle_count(gltf_path: Path) -> int:
    """从 glTF 文档实算三角形计数（索引 accessor count/3 求和）。"""
    doc = json.loads(gltf_path.read_text(encoding="utf-8"))
    accessors = doc.get("accessors", [])
    tris = 0
    for mesh in doc.get("meshes", []):
        for prim in mesh.get("primitives", []):
            if prim.get("mode", 4) != 4:
                continue
            if "indices" in prim:
                tris += accessors[prim["indices"]].get("count", 0) // 3
            else:
                pos = prim.get("attributes", {}).get("POSITION")
                if pos is not None:
                    tris += accessors[pos].get("count", 0) // 3
    return tris


def gltf_table_counts(gltf_path: Path) -> dict[str, int]:
    doc = json.loads(gltf_path.read_text(encoding="utf-8"))
    return {k: len(doc.get(k, [])) for k in ("scenes", "nodes", "meshes", "materials", "textures")}


def validate_counts(counts: dict) -> list[str]:
    """计数非空判据（三角形/材质/纹理）；纯函数供 selftest 红臂复用。"""
    fails: list[str] = []
    for k in ("triangles", "materials", "textures"):
        v = counts.get(k)
        if not isinstance(v, int) or v < 1:
            fails.append(f"{k} 计数为零/缺失（计数为零冒充成功即 RED）: {v!r}")
    return fails


def reconcile_scene_sets(ready: list[str], loaded: list[str]) -> list[str]:
    """静默丢场景判据；纯函数供 selftest 红臂复用。"""
    fails: list[str] = []
    missing = sorted(set(ready) - set(loaded))
    extra = sorted(set(loaded) - set(ready))
    if missing:
        fails.append(f"静默丢场景（ready 而未加载）: {missing}")
    if extra:
        fails.append(f"加载集含清单外场景: {extra}")
    return fails


def load_scene(exe: Path, gltf_path: Path) -> tuple[dict | None, list[str]]:
    """逐场景加载协议；返回 (结果行, 事件序列)。"""
    events: list[str] = []
    row: dict = {"gltf": str(gltf_path)}
    rc, out = run_cmd([str(exe), "import-gltf", str(gltf_path), "--emit-digest"])
    if rc != 0 or "status=ok" not in out:
        check(False, f"{gltf_path.name}: rxcook import rc={rc} 非零/无 status=ok")
        return None, events
    events.append("rxcook_import_ok")
    if "coverage_complete=true" not in out:
        check(False, f"{gltf_path.name}: coverage_complete≠true（静默丢字段）")
        return None, events
    events.append("coverage_complete")
    tables = extract_tables(out)
    if tables is None or any(t not in tables for t in TABLES):
        check(False, f"{gltf_path.name}: 六表提取失败")
        return None, events
    events.append("tables_extracted")
    counts = gltf_table_counts(gltf_path)
    counts["triangles"] = gltf_triangle_count(gltf_path)
    row["counts"] = counts
    row["tables"] = tables
    cfails = validate_counts(counts)
    for f in cfails:
        check(False, f"{gltf_path.name}: {f}")
    if not cfails:
        events.append("counts_nonzero")
    return row, events


def leg_red_fixtures() -> bool:
    ok_all = True
    fixtures = sorted(RED_FIXTURE_DIR.glob("*.json"))
    if len(fixtures) < 3:
        check(False, f"M132 RED 语料不足 3 件: {len(fixtures)}")
        return False
    real_manifest = lib.load_json(lib.MANIFEST_PATH)
    for fp in fixtures:
        doc = lib.load_json(fp)
        name = fp.stem
        if name == "red_zero_count_masquerade":
            fails = validate_counts(doc.get("counts", {}))
            if not fails:
                check(False, f"{name}: 零计数冒充未被计数判据检出")
                ok_all = False
        elif name == "red_silent_scene_drop":
            fails = reconcile_scene_sets(doc.get("ready", []), doc.get("loaded", []))
            if not fails:
                check(False, f"{name}: 静默丢场景未被对账判据检出")
                ok_all = False
        elif name == "red_all_not_ready_manifest":
            fails = lib.validate_manifest(doc, lib.load_json(lib.REGISTRY_PATH))
            if not any("vacuous" in f or "ready 场景数" in f for f in fails):
                check(False, f"{name}: 全 not-ready vacuous 未被 ready 下界拦截: {fails[:2]}")
                ok_all = False
        else:
            check(False, f"未知 RED 语料（闭集外）: {name}")
            ok_all = False
    note("M132 RED 三件全检出（零计数冒充/静默丢场景/vacuous 空清单）" if ok_all else "M132 RED 检出不全")
    return ok_all


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if not validate_counts({"triangles": 0, "materials": 1, "textures": 1}):
        print(f"[{TAG}] selftest FAIL: 零三角形未判红", file=sys.stderr)
        return 1
    if not reconcile_scene_sets(["a", "b"], ["a"]):
        print(f"[{TAG}] selftest FAIL: 静默丢场景未判红", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    device_state = "not_applicable"
    scene_results: list[dict] = []

    registry = lib.load_json(lib.REGISTRY_PATH) if lib.REGISTRY_PATH.is_file() else {}
    manifest = lib.load_json(lib.MANIFEST_PATH) if lib.MANIFEST_PATH.is_file() else {}
    golden = lib.load_json(GOLDEN_PATH) if GOLDEN_PATH.is_file() else None
    if golden is None:
        check(False, f"缺加载 golden {GOLDEN_PATH.name}（P-09：golden 由实测首跑冻结）")

    m_fails = lib.validate_manifest(manifest, registry) if manifest and registry else ["清单/注册表缺失"]
    for f in m_fails:
        check(False, f"清单校验: {f}")
    checks["manifest_valid"] = not m_fails

    root, src = lib.resolve_cache_root()
    note(f"缓存根解析: {src}")
    checks["cache_root_resolved"] = root is not None
    if root is None:
        device_state = "dev_env_degrade"
        check(False, f"缓存根不可达（fail-closed）: {src}")

    ready_scenes = [r for r in manifest.get("scenes", []) if r.get("status") == "ready"]
    assets_by_id = {a.get("asset_id"): a for a in registry.get("assets", [])}

    if root is not None and checks["manifest_valid"]:
        digest_ok = True
        for row in ready_scenes:
            asset = assets_by_id.get(row["asset_id"])
            if asset is None:
                check(False, f"{row['scene_id']}: asset 未登记")
                digest_ok = False
                continue
            vf = lib.verify_asset_cache(asset, root)
            for f in vf:
                check(False, f"{row['scene_id']} 加载前 digest 核验: {f}")
                digest_ok = False
        checks["per_scene_digest_verified"] = digest_ok

        exe = build_rxcook()
        import_ok = True
        counts_ok = True
        golden_ok = True
        events_ok = True
        if exe is not None and golden is not None:
            golden_scenes = golden.get("scenes", {})
            for row in ready_scenes:
                sid = row["scene_id"]
                asset = assets_by_id[row["asset_id"]]
                load_unit = asset.get("load_unit")
                if not load_unit:
                    check(False, f"{sid}: 资产登记缺 load_unit")
                    import_ok = False
                    continue
                gltf_path = root / str(asset["cache_rel"]) / load_unit
                res, events = load_scene(exe, gltf_path)
                entry = {"scene_id": sid, "events": events, "ok": res is not None}
                if res is not None:
                    entry["counts"] = res["counts"]
                    entry["table_digests"] = {t: res["tables"][t].get("digest") for t in TABLES}
                scene_results.append(entry)
                if res is None:
                    import_ok = False
                    continue
                g = golden_scenes.get(sid)
                if g is None:
                    check(False, f"{sid}: golden 缺场景行")
                    golden_ok = False
                    continue
                if res["counts"] != g.get("counts"):
                    check(False, f"{sid}: 计数 ≠ golden（{res['counts']} ≠ {g.get('counts')}）")
                    golden_ok = False
                for t in TABLES:
                    if res["tables"][t].get("count") != g.get("tables", {}).get(t, {}).get("count"):
                        check(False, f"{sid}: 六表 {t}.count ≠ golden")
                        golden_ok = False
                    if res["tables"][t].get("digest") != g.get("tables", {}).get(t, {}).get("digest"):
                        check(False, f"{sid}: 六表 {t}.digest ≠ golden")
                        golden_ok = False
                if events != g.get("event_sequence"):
                    check(False, f"{sid}: 加载事件序列 ≠ golden（{events} ≠ {g.get('event_sequence')}）")
                    events_ok = False
                if not validate_counts(res["counts"]):
                    counts_ok = counts_ok and True
                else:
                    counts_ok = False
        checks["per_scene_import_ok"] = import_ok
        checks["counts_nonzero"] = counts_ok
        checks["counts_equal_golden"] = golden_ok
        checks["event_sequence_golden"] = events_ok

        loaded = [e["scene_id"] for e in scene_results if e.get("ok")]
        r_fails = reconcile_scene_sets([r["scene_id"] for r in ready_scenes], loaded)
        for f in r_fails:
            check(False, f"场景对账: {f}")
        checks["scene_set_reconciled"] = not r_fails

    checks["red_fixtures_all_detected"] = leg_red_fixtures()

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES and device_state == "not_applicable"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "scenes": scene_results,
        "commands": COMMANDS,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（逐场景加载绿 + 计数非空 + golden 全等 + 事件序列 golden + 静默丢场景零 + RED 三件全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
