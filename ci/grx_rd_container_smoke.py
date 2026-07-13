#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Route B S1: RD-native container structural CI smoke (host-only, no GPU).

This harness productionizes the ``spike/godot-rurix/rd-native-pipeline`` container
generator into a fail-closed CI gate. It batch-builds a
``RenderingShaderContainerD3D12`` container for every offline compute pass and
then independently self-checks each one — purely at the byte/structure level. It
NEVER touches a GPU, a Godot binary, or a D3D12 device, so it can run in any
environment and never collides with a build/GPU job.

Mirrors the grx009..013 CI-script shape (module docstring, LF-only evidence JSON,
``rel()`` paths, machine-readable outcome) but is strictly structural: the
GPU/runtime consumption of these containers is proven separately by the S2
``ci/grx_rd_native_probe_smoke.py`` probe (tonemap texture path, ~1 ULP).

Fail-closed drift gate. Each pass/kernel is PINNED to an expected outcome:

  * ``container``   — the layout is route-B-representable (sampler-free, single
    set, single descriptor table, b0 <= 128B), so a container must be produced
    AND pass every structural self-check.
  * ``fail_closed`` — the layout is NOT representable by this spike and the
    generator must reject it with the pinned category, NOT silently coerce or
    drop it (route_b_plan R1/R4/R7). Today the only rejected passes are
    gpu_culling (144B b0) and indirect_args write+validate (176B b0), both
    ``push_constant_too_large`` (they must migrate b0 to a CBV first).

The gate FAILs on ANY drift: a route-B pass that stops producing a valid
container, an unsupported pass that silently starts producing one, a wrong
fail-closed category, a self-check failure, a missing/extra kernel, or a CRLF
byte sneaking into a generated report. Everything else is a green, reproducible
structural pass.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import importlib.util
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PIPELINE_DIR = ROOT / "spike" / "godot-rurix" / "rd-native-pipeline"
PASSES_ROOT = ROOT / "spike" / "godot-rurix" / "passes"
OUT_DIR = PIPELINE_DIR / "out"
EVIDENCE_OUT = PIPELINE_DIR / "rd_container_smoke_evidence.json"

SUBJECT = "grx_rd_container_smoke"


def _load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {name} from {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


gen = _load_module("rd_generate_rd_container", PIPELINE_DIR / "generate_rd_container.py")
ver = _load_module("rd_verify_container", PIPELINE_DIR / "verify_container.py")


# Pinned expected outcome per produced kernel (keyed by out_stem). Drift from
# this table is a hard FAIL. The two fail-closed entries are the honest record of
# the >128B push-constant passes; if a future artifact migrates b0 to a CBV, flip
# its entry to ("container", None) in the SAME change that adds the cbuffer path.
EXPECT: dict[str, tuple[str, str | None]] = {
    "tonemap": ("container", None),
    "taa_resolve": ("container", None),
    "ssao_blur": ("container", None),
    "particles_copy": ("container", None),
    "luminance_reduction": ("container", None),
    "cluster_store": ("container", None),
    "gpu_culling": ("fail_closed", "push_constant_too_large"),
    "fused_post_chain": ("container", None),
    "instance_compaction_scan_local": ("container", None),
    "instance_compaction_scan_groups": ("container", None),
    "instance_compaction_scatter": ("container", None),
    "indirect_args_write": ("fail_closed", "push_constant_too_large"),
    "indirect_args_validate": ("fail_closed", "push_constant_too_large"),
}


def now_iso() -> str:
    return _dt.datetime.now().astimezone().replace(microsecond=0).isoformat()


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT)).replace("\\", "/")
    except ValueError:
        return str(path)


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # Byte-level LF only (repo .gitattributes pins `* -text`); never emit CRLF.
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def effective_resources_by_stem() -> dict[str, list]:
    """Re-enumerate the registry so each kernel's effective layout resource list
    (variant kernels pull it from layout["variants"][i]["resources"]) is available
    for the per-container self-check, keyed by out_stem."""
    out: dict[str, list] = {}
    kernel_paths: dict[str, dict] = {}
    for spec in gen.PASS_REGISTRY:
        pass_id = spec["pass_id"]
        artifacts_dir = PASSES_ROOT / pass_id / "artifacts"
        layout_path = artifacts_dir / (f"{pass_id}_descriptor_layout.json")
        layout = gen.load_layout(layout_path)
        for job in gen.enumerate_kernels(spec, artifacts_dir, layout):
            out[job["out_stem"]] = job["layout"].get("resources", [])
            kernel_paths[job["out_stem"]] = {"dxil": job["dxil"], "rts0": job["rts0"]}
    return out, kernel_paths


def main() -> int:
    print(f"[{SUBJECT}] host-only structural smoke (no GPU, no engine)")
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    failures: list[str] = []

    # --- 1. batch-produce every container ---------------------------------
    results = gen.generate_all(PASSES_ROOT, OUT_DIR)
    by_stem: dict[str, dict] = {}
    for pe in results:
        for k in pe["kernels"]:
            stem = k.get("out_stem") or k.get("label")
            by_stem[stem] = k

    # --- 2. drift check vs the pinned expectations ------------------------
    seen = set(by_stem)
    pinned = set(EXPECT)
    for missing in sorted(pinned - seen):
        failures.append(f"expected kernel {missing!r} was not produced by the generator")
    for extra in sorted(seen - pinned):
        failures.append(f"generator produced unpinned kernel {extra!r}; add it to EXPECT")

    resources_by_stem, kernel_paths = effective_resources_by_stem()

    kernel_reports: list[dict] = []
    verify_total = 0
    container_count = 0
    fail_closed_count = 0

    for stem in sorted(pinned & seen):
        exp_status, exp_category = EXPECT[stem]
        got = by_stem[stem]
        entry: dict = {
            "kernel": stem,
            "expected_status": exp_status,
            "actual_status": got["status"],
        }

        if got["status"] != exp_status:
            entry["drift"] = (
                f"status {got['status']!r} != expected {exp_status!r} "
                f"({got.get('reason') or got.get('category', '')})"
            )
            failures.append(f"{stem}: {entry['drift']}")
            kernel_reports.append(entry)
            continue

        if exp_status == "fail_closed":
            fail_closed_count += 1
            entry["category"] = got.get("category")
            entry["reason"] = got.get("reason")
            if got.get("category") != exp_category:
                msg = (
                    f"{stem}: fail-closed category {got.get('category')!r} != "
                    f"expected {exp_category!r}"
                )
                entry["drift"] = msg
                failures.append(msg)
            kernel_reports.append(entry)
            continue

        # exp_status == "container": self-check the produced bytes.
        container_count += 1
        container_path = Path(got["container_path"])
        entry["container"] = rel(container_path)
        entry["size_bytes"] = got.get("size")
        entry["push_constant_size"] = got.get("push_constant_size")
        entry["resource_descriptor_count"] = got.get("resource_descriptor_count")
        entry["container_sha256"] = sha256_file(container_path)

        if not container_path.is_file():
            failures.append(f"{stem}: container path {rel(container_path)} does not exist")
            kernel_reports.append(entry)
            continue

        paths = kernel_paths.get(stem, {})
        dxil_path = Path(paths.get("dxil"))
        rts0_path = Path(paths.get("rts0"))
        resources = resources_by_stem.get(stem, [])
        count, verify_failures = ver.verify_container_file(
            container_path, dxil_path, rts0_path, resources, quiet=True
        )
        verify_total += count
        entry["verify_checks"] = count
        entry["verify_failures"] = verify_failures
        if verify_failures:
            for vf in verify_failures:
                failures.append(f"{stem}: self-check FAIL: {vf}")

        # Generated report sidecar must be LF-clean (repo .gitattributes `* -text`).
        report_path = container_path.with_suffix(".report.json")
        if report_path.is_file() and b"\r" in report_path.read_bytes():
            failures.append(f"{stem}: report {rel(report_path)} contains CR bytes")

        kernel_reports.append(entry)

    # --- 3. emit evidence + summary ---------------------------------------
    status = "pass" if not failures else "fail"
    doc = {
        "schema_version": 1,
        "subject": SUBJECT,
        "segment": "route_b_s1_rd_container_pipeline",
        "status": status,
        "timestamp": now_iso(),
        "run_url": github_run_url(),
        "host_only": True,
        "gpu_used": False,
        "real_gpu_pass": False,
        "note": (
            "Structural-only CI smoke for the route-B RD-native container "
            "generator. Batch-builds a RenderingShaderContainerD3D12 container per "
            "offline compute pass and independently self-checks each one at the "
            "byte level. Layouts that exceed the 128B root-constant window "
            "(gpu_culling, indirect_args) are pinned as expected fail-closed. No "
            "GPU/engine is touched; runtime consumption is proven separately by "
            "the S2 probe (ci/grx_rd_native_probe_smoke.py)."
        ),
        "counts": {
            "containers_produced": container_count,
            "fail_closed": fail_closed_count,
            "verify_checks_total": verify_total,
            "kernels_pinned": len(EXPECT),
        },
        "kernels": kernel_reports,
        "failures": failures,
    }
    _write_json(EVIDENCE_OUT, doc)

    print(f"[{SUBJECT}] {container_count} container(s), {fail_closed_count} fail-closed, "
          f"{verify_total} self-check assertions")
    for entry in kernel_reports:
        drifted = "drift" in entry
        vf = entry.get("verify_failures") or []
        tag = "FAIL" if (drifted or vf) else "ok "
        if drifted:
            print(f"  [{tag}] {entry['kernel']:<32} DRIFT: {entry['drift']}")
        elif entry["actual_status"] == "container" and entry.get("size_bytes") is not None:
            checks = entry.get("verify_checks")
            print(f"  [{tag}] {entry['kernel']:<32} {entry['size_bytes']:>6}B "
                  f"pc={entry.get('push_constant_size')}B verify={checks}/{checks}")
        else:
            print(f"  [{tag}] {entry['kernel']:<32} fail-closed ({entry.get('category')})")

    print(f"[{SUBJECT}] wrote {rel(EVIDENCE_OUT)} status={status}")
    if failures:
        print(f"[{SUBJECT}] FAIL: {len(failures)} drift/self-check violation(s)", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(f"[{SUBJECT}] PASS: all {len(EXPECT)} kernels match their pinned outcome")
    return 0


if __name__ == "__main__":
    sys.exit(main())
