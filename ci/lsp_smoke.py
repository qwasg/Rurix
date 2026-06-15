# -*- coding: utf-8 -*-
"""G-M6-2/G-M6-5 LSP MVP 能力面冒烟(M6 CI_GATES 步骤 28).

真跑 `rurixc --tooling-smoke` 与 `rurixc --tooling-server` JSON-RPC 往返,
在 `conformance/toolchain/lsp_mvp/sample.rx` 上验证六项 LSP 能力
(publishDiagnostics/completion/definition/references/highlight/rename);
内嵌红绿:篡改预期 completion 标签后脚本 FAIL,复原后 PASS。
成功写 evidence/lsp_smoke_*.json,计入 m6.counter.lsp_capabilities。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "conformance" / "toolchain" / "lsp_mvp" / "sample.rx"
RURIXC = ROOT / "target" / "debug" / ("rurixc.exe" if os.name == "nt" else "rurixc")

CAPABILITY_ORDER = [
    "publishDiagnostics",
    "completion",
    "definition",
    "references",
    "highlight",
    "rename",
]


def run(
    cmd: list[str],
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd, cwd=cwd or ROOT, capture_output=True, text=True, env=env
    )


def unique_evidence_path() -> Path:
    base = ROOT / "evidence"
    base.mkdir(parents=True, exist_ok=True)
    stem = f"lsp_smoke_{_dt.datetime.now():%Y%m%d_%H%M%S}"
    out = base / f"{stem}.json"
    n = 1
    while out.exists():
        out = base / f"{stem}_{n}.json"
        n += 1
    return out


def smoke_once(extra_env: dict[str, str] | None = None) -> tuple[dict, subprocess.CompletedProcess[str]]:
    env = os.environ.copy()
    env.pop("RURIX_LSP_SMOKE_EXPECT_COMPLETION", None)
    if extra_env:
        env.update(extra_env)
    proc = run([str(RURIXC), "--tooling-smoke", str(FIXTURE)], env=env)
    try:
        payload = json.loads(proc.stdout.strip() or "{}")
    except json.JSONDecodeError:
        payload = {"capabilities_passed": [], "failures": [proc.stdout + proc.stderr], "ok": False}
    return payload, proc


def frame_lsp(message: dict) -> bytes:
    body = json.dumps(message, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return f"Content-Length: {len(body)}\r\n\r\n".encode("ascii") + body


def parse_lsp_frames(raw: bytes) -> list[dict]:
    out: list[dict] = []
    pos = 0
    while pos < len(raw):
        header_end = raw.find(b"\r\n\r\n", pos)
        sep_len = 4
        if header_end < 0:
            header_end = raw.find(b"\n\n", pos)
            sep_len = 2
        if header_end < 0:
            break
        header = raw[pos:header_end].decode("ascii", errors="replace")
        length = None
        for line in header.splitlines():
            if line.lower().startswith("content-length:"):
                length = int(line.split(":", 1)[1].strip())
                break
        if length is None:
            break
        body_start = header_end + sep_len
        body = raw[body_start : body_start + length]
        if len(body) != length:
            break
        out.append(json.loads(body.decode("utf-8")))
        pos = body_start + length
    return out


def line_col(src: str, needle: str, bias: int = 0) -> tuple[int, int]:
    off = src.index(needle) + bias
    before = src[:off]
    line = before.count("\n")
    col = off - (before.rfind("\n") + 1 if "\n" in before else 0)
    return line, col


def server_roundtrip() -> tuple[dict, subprocess.CompletedProcess[bytes]]:
    src = FIXTURE.read_text(encoding="utf-8")
    foo_line, foo_col = line_col(src, "foo", 1)
    helper_line, helper_col = line_col(src, "helper()", 1)
    payload = b"".join(
        frame_lsp(msg)
        for msg in [
            {"jsonrpc": "2.0", "id": "init", "method": "initialize", "params": {}},
            {
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": "file:///sample.rx",
                        "version": 1,
                        "text": src,
                    }
                },
            },
            {
                "jsonrpc": "2.0",
                "id": "completion",
                "method": "textDocument/completion",
                "params": {
                    "textDocument": {"uri": "file:///sample.rx"},
                    "position": {"line": foo_line, "character": foo_col},
                },
            },
            {
                "jsonrpc": "2.0",
                "id": "definition",
                "method": "textDocument/definition",
                "params": {
                    "textDocument": {"uri": "file:///sample.rx"},
                    "position": {"line": helper_line, "character": helper_col},
                },
            },
            {
                "jsonrpc": "2.0",
                "id": "highlight",
                "method": "textDocument/documentHighlight",
                "params": {
                    "textDocument": {"uri": "file:///sample.rx"},
                    "position": {"line": foo_line, "character": foo_col},
                },
            },
            {
                "jsonrpc": "2.0",
                "id": "rename_bad",
                "method": "textDocument/rename",
                "params": {
                    "textDocument": {"uri": "file:///sample.rx"},
                    "position": {"line": foo_line, "character": foo_col},
                    "newName": "fn",
                },
            },
        ]
    )
    proc = subprocess.run(
        [str(RURIXC), "--tooling-server"],
        input=payload,
        capture_output=True,
        timeout=10,
    )
    frames = parse_lsp_frames(proc.stdout)
    by_id = {f.get("id"): f for f in frames if "id" in f}
    diagnostics = [f for f in frames if f.get("method") == "textDocument/publishDiagnostics"]
    failures: list[str] = []
    if proc.returncode != 0:
        failures.append(f"server exit={proc.returncode} stderr={proc.stderr.decode(errors='replace')}")
    caps = by_id.get("init", {}).get("result", {}).get("capabilities", {})
    for key in ["completionProvider", "definitionProvider", "referencesProvider", "documentHighlightProvider", "renameProvider"]:
        if key not in caps:
            failures.append(f"initialize missing {key}")
    completion_items = by_id.get("completion", {}).get("result", {}).get("items", [])
    if not any(i.get("label") == "foo" and i.get("kind") for i in completion_items):
        failures.append("server completion did not return foo with kind")
    if not by_id.get("definition", {}).get("result", {}).get("range"):
        failures.append("server definition missing range")
    highlights = by_id.get("highlight", {}).get("result", [])
    if not highlights or not all("range" in h for h in highlights):
        failures.append("server documentHighlight missing DocumentHighlight range objects")
    rename = by_id.get("rename_bad", {}).get("result", {}).get("changes", {})
    if rename.get("file:///sample.rx") != []:
        failures.append("server invalid rename did not return empty WorkspaceEdit")
    if not any("RX7012" in json.dumps(d, ensure_ascii=False) for d in diagnostics):
        failures.append("server invalid rename did not publish RX7012 diagnostic")
    return {"ok": not failures, "failures": failures, "frame_count": len(frames)}, proc


def main() -> int:
    emit = "--no-emit" not in sys.argv
    build = run(["cargo", "build", "-p", "rurixc"])
    if build.returncode != 0:
        print(f"[lsp_smoke] FAIL: cargo build -p rurixc:\n{build.stderr}")
        return 1
    if not RURIXC.is_file():
        print(f"[lsp_smoke] FAIL: rurixc 产物不存在: {RURIXC}")
        return 1
    if not FIXTURE.is_file():
        print(f"[lsp_smoke] FAIL: fixture 缺失: {FIXTURE}")
        return 1

    green, green_proc = smoke_once()
    if green_proc.returncode != 0 or not green.get("ok"):
        print(
            f"[lsp_smoke] FAIL: 绿路径失败 exit={green_proc.returncode} "
            f"payload={green}\nstderr={green_proc.stderr}"
        )
        return 1
    passed = sorted(set(green.get("capabilities_passed", [])))
    if len(passed) < 5:
        print(f"[lsp_smoke] FAIL: capabilities_passed={passed} 不足 5 项")
        return 1

    server, server_proc = server_roundtrip()
    if server_proc.returncode != 0 or not server.get("ok"):
        print(
            f"[lsp_smoke] FAIL: --tooling-server 往返失败 payload={server}\n"
            f"stdout={server_proc.stdout.decode(errors='replace')}\n"
            f"stderr={server_proc.stderr.decode(errors='replace')}"
        )
        return 1

    # 红绿:强制失败预期(应红却绿即脚本 FAIL)
    red, red_proc = smoke_once({"RURIX_LSP_SMOKE_EXPECT_COMPLETION": "___no_such_symbol___"})
    if red_proc.returncode == 0 and red.get("ok"):
        print("[lsp_smoke] FAIL: 红绿门失效 — 篡改预期后仍 PASS")
        return 1

    red2, red2_proc = smoke_once()
    if red2_proc.returncode != 0 or not red2.get("ok"):
        print(f"[lsp_smoke] FAIL: 复原后未转绿 payload={red2}")
        return 1

    if emit:
        out = unique_evidence_path()
        doc = {
            "schema_version": 1,
            "subject": "lsp_capabilities",
            "timestamp": _dt.datetime.now(_dt.timezone.utc).isoformat(),
            "capabilities_passed": passed,
            "facts": [
                {
                    "command": f"{RURIXC.name} --tooling-smoke {FIXTURE.relative_to(ROOT)}",
                    "exit_code": green_proc.returncode,
                    "capabilities_passed": passed,
                },
                {
                    "command": f"{RURIXC.name} --tooling-server",
                    "exit_code": server_proc.returncode,
                    "capabilities_passed": passed,
                    "frame_count": server["frame_count"],
                }
            ],
            "redgreen": {
                "tamper": "RURIX_LSP_SMOKE_EXPECT_COMPLETION=___no_such_symbol___",
                "red_exit_code": red_proc.returncode,
                "green_exit_code": red2_proc.returncode,
            },
        }
        out.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"[lsp_smoke] PASS: {len(passed)} capabilities -> {out.relative_to(ROOT)}")
    else:
        print(f"[lsp_smoke] PASS (no-emit): {len(passed)} capabilities")
    return 0


if __name__ == "__main__":
    sys.exit(main())
