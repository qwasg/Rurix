# -*- coding: utf-8 -*-
"""G-M6-1 三包 workspace 离线重建逐字节可复现门.

真跑 `rx build --manifest-path ... --locked --offline` 两次,比较 host EXE
SHA-256、rurix.lock SHA-256 与 vendor 内容树哈希;随后临时篡改 vendor/pathdep
确认 RX7008 红,复原后转绿。成功写 evidence/offline_rebuild_*.json,供
m6.counter.offline_rebuild_reproducible 计数。
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "conformance" / "workspace" / "repro"
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")


def run(cmd: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def sha256_tree(root: Path) -> str:
    h = hashlib.sha256()
    files = sorted(p for p in root.rglob("*") if p.is_file())
    for p in files:
        rel = p.relative_to(root).as_posix().encode("utf-8")
        data = p.read_bytes()
        h.update(len(rel).to_bytes(8, "little"))
        h.update(rel)
        h.update(len(data).to_bytes(8, "little"))
        h.update(data)
    return h.hexdigest()


def unique_evidence_path() -> Path:
    base = ROOT / "evidence"
    base.mkdir(parents=True, exist_ok=True)
    stem = f"offline_rebuild_{_dt.datetime.now():%Y%m%d_%H%M%S}"
    out = base / f"{stem}.json"
    n = 1
    while out.exists():
        out = base / f"{stem}_{n}.json"
        n += 1
    return out


def build_once(workspace: Path, run_id: str) -> tuple[dict, subprocess.CompletedProcess[str]]:
    out_dir = workspace / "build" / "offline_rebuild"
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)
    exe = out_dir / "repro_app.exe"
    cmd = [
        str(RX),
        "build",
        "--manifest-path",
        str(workspace / "rurix.toml"),
        "--locked",
        "--offline",
        "-o",
        str(exe),
    ]
    proc = run(cmd, ROOT)
    fact = {
        "run_id": run_id,
        "command": " ".join(cmd),
        "exit_code": proc.returncode,
        "stdout": proc.stdout[-1000:],
        "stderr": proc.stderr[-1000:],
    }
    if proc.returncode == 0:
        fact["artifact_sha256"] = sha256_file(exe)
        fact["lock_sha256"] = sha256_file(workspace / "rurix.lock")
        fact["vendor_sha256"] = sha256_tree(workspace / "vendor")
    return fact, proc


def main() -> int:
    emit = "--no-emit" not in sys.argv
    build = run(["cargo", "build", "-p", "rx"], ROOT)
    if build.returncode != 0:
        print(f"[offline_rebuild] FAIL: cargo build -p rx 失败:\n{build.stderr}")
        return 1
    if not RX.is_file():
        print(f"[offline_rebuild] FAIL: rx 产物不存在: {RX}")
        return 1
    if not (FIXTURE / "rurix.lock").is_file():
        print("[offline_rebuild] FAIL: fixture 缺 conformance/workspace/repro/rurix.lock")
        return 1
    if not (FIXTURE / "vendor" / "pathdep" / "rurix.toml").is_file():
        print("[offline_rebuild] FAIL: fixture 缺 vendor/pathdep 快照")
        return 1

    tmp_root = ROOT / "build" / "offline_rebuild_tmp"
    if tmp_root.exists():
        shutil.rmtree(tmp_root)
    tmp_root.mkdir(parents=True)
    workspace = tmp_root / "repro"
    shutil.copytree(FIXTURE, workspace)

    facts: list[dict] = []
    first, proc1 = build_once(workspace, "first")
    facts.append(first)
    second, proc2 = build_once(workspace, "second")
    facts.append(second)
    if proc1.returncode != 0 or proc2.returncode != 0:
        print("[offline_rebuild] FAIL: 两次 locked/offline build 未全部通过")
        for fact in facts:
            print(json.dumps(fact, ensure_ascii=False, indent=2))
        return 1

    artifact_hashes = [first["artifact_sha256"], second["artifact_sha256"]]
    lock_hashes = [first["lock_sha256"], second["lock_sha256"]]
    vendor_hashes = [first["vendor_sha256"], second["vendor_sha256"]]
    if len(set(artifact_hashes)) != 1 or len(set(lock_hashes)) != 1 or len(set(vendor_hashes)) != 1:
        print("[offline_rebuild] FAIL: 两次重建哈希不一致")
        print(json.dumps(facts, ensure_ascii=False, indent=2))
        return 1

    tampered = workspace / "vendor" / "pathdep" / "src" / "lib.rx"
    original = tampered.read_bytes()
    tampered.write_bytes(original + b"\n// tampered by offline_rebuild_repro.py\n")
    red_fact, red_proc = build_once(workspace, "tamper-red")
    facts.append(red_fact)
    red_output = red_proc.stdout + red_proc.stderr
    red_ok = red_proc.returncode != 0 and "RX7008" in red_output
    tampered.write_bytes(original)
    green_fact, green_proc = build_once(workspace, "restore-green")
    facts.append(green_fact)
    green_ok = green_proc.returncode == 0
    if not red_ok or not green_ok:
        print("[offline_rebuild] FAIL: 篡改红绿验证失败")
        print(json.dumps({"red_ok": red_ok, "green_ok": green_ok, "facts": facts}, ensure_ascii=False, indent=2))
        return 1

    doc = {
        "schema_version": 1,
        "subject": "offline_rebuild_reproducible",
        "reproducible": True,
        "fixture": "conformance/workspace/repro",
        "package_sources": ["path", "git", "archive"],
        "artifact_sha256_runs": artifact_hashes,
        "lock_sha256_runs": lock_hashes,
        "vendor_sha256_runs": vendor_hashes,
        "redgreen": {
            "tamper": "vendor/pathdep/src/lib.rx",
            "red_exit_code": red_proc.returncode,
            "red_code": "RX7008",
            "green_exit_code": green_proc.returncode,
        },
        "rx_binary": str(RX.relative_to(ROOT)).replace("\\", "/"),
        "facts": facts,
        "timestamp": _dt.datetime.now().astimezone().isoformat(timespec="seconds"),
    }
    if emit:
        out = unique_evidence_path()
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[offline_rebuild] evidence 写入 {out.relative_to(ROOT)}")
    print("[offline_rebuild] PASS: 两次 EXE SHA-256 一致,lock/vendor 不变,篡改 RX7008 红绿通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
