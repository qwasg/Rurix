#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Route B S2 — zero-patch in-engine RD-native compute probe smoke.

Drives the tracked Godot template build (external/godot-master/bin, patches
0001+0002+0003 only — none of which touch the RenderingDevice public API) over
the GDScript probe project in
``spike/godot-rurix/rd-native-pipeline/probe_project/``. The probe loads the
Rurix-built RenderingShaderContainerD3D12 container
(``out/tonemap.rd_container.bin``, produced by ../generate_rd_container.py from
the Rurix tonemap .dxil + .rts0.bin verbatim) through the public
``RenderingDevice.shader_create_from_bytecode`` surface, creates the compute
pipeline (driver ``CreateRootSignature`` + PSO from the container bytes), binds
a seeded source Texture2D (t0 SRV) and a storage RWTexture2D (u0 UAV), dispatches
the compute list on a *local* rendering device, and reads back the result.

This harness owns the CPU reference (the same
``linear_to_srgb(src * luminance_multiplier * exposure)`` formula and binary32
rounding as spike/godot-rurix/passes/tonemap/generate_math_parity_evidence.py)
and compares the GPU readback per texel within the same 2e-3 tolerance.

It proves ONE thing: the same Rurix GPU-side bytes that the offline dispatch
smoke exercises through a hand-built D3D12 harness also run, unmodified, as a
first-class RenderingDevice compute pass inside the Godot engine WITHOUT any
engine patch. It does NOT claim same-frame injection, default pass enablement,
real_gpu_pass=true, visual/telemetry evidence, or any performance number.

Fail-closed discipline (mirrors ci/grx010_tonemap_d3d12_dispatch_smoke.py):
  * The device is always real. If no usable D3D12 device exists, the run
    records ``status=skip`` with a concrete reason and exits 0 (exit 1 when
    RURIX_REQUIRE_REAL=1). SKIP never advances any gate.
  * A device that IS present but where the container load, pipeline creation,
    dispatch, readback, or per-texel comparison fails is ``status=fail``.
  * The container is (re)built from the tracked tonemap artifacts if missing;
    its sha256 and the source DXIL sha256 are recorded either way.

Evidence: spike/godot-rurix/rd-native-pipeline/rd_native_probe_evidence.json
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PIPELINE_DIR = ROOT / "spike" / "godot-rurix" / "rd-native-pipeline"
PROBE_PROJECT = PIPELINE_DIR / "probe_project"
GENERATOR = PIPELINE_DIR / "generate_rd_container.py"
CONTAINER = PIPELINE_DIR / "out" / "tonemap.rd_container.bin"
TONEMAP_ARTIFACTS = ROOT / "spike" / "godot-rurix" / "passes" / "tonemap" / "artifacts"
TONEMAP_DXIL = TONEMAP_ARTIFACTS / "tonemap.dxil"
TONEMAP_RTS0 = TONEMAP_ARTIFACTS / "tonemap.rts0.bin"
EVIDENCE_OUT = PIPELINE_DIR / "rd_native_probe_evidence.json"
WORK = ROOT / "target" / "grx_rd_native_probe"

DEFAULT_GODOT_EXE = (
    ROOT / "external" / "godot-master" / "bin" / "godot.windows.template_debug.x86_64.console.exe"
)
# Env fallbacks for the Godot console exe (below --godot-exe, above the tracked
# default). RURIX_PROBE_GODOT_EXE is probe-specific; RURIX_BENCH_GODOT_EXE is
# shared with the bench runner so one build can drive both.
ENV_GODOT_EXE = ("RURIX_PROBE_GODOT_EXE", "RURIX_BENCH_GODOT_EXE")

SUBJECT = "grx_rd_native_probe"
TIMEOUT_SECONDS = 300
# Same tolerance as the tonemap math-parity evidence (absorbs GPU pow()
# approximation vs the binary32 CPU reference).
MAX_ABS_ERROR_TOLERANCE = 2e-3

# Four cases identical to
# spike/godot-rurix/passes/tonemap/generate_math_parity_evidence.py, so this
# smoke and the offline math-parity evidence share one口径.
# (case_id, width, height, exposure, white, luminance_multiplier)
CASES = [
    ("tonemap_8x8_exposure1", 8, 8, 1.0, 1.0, 1.0),
    ("tonemap_8x8_exposure_half", 8, 8, 0.5, 1.0, 1.0),
    ("tonemap_16x9_lum_mult2", 16, 9, 1.0, 4.0, 2.0),
    ("tonemap_9x7_partial_tiles", 9, 7, 1.25, 1.0, 0.75),
]

DOES_NOT_IMPLY = [
    "same-frame injection (that is S3+, requires a patch)",
    "default pass enablement",
    "real_gpu_pass=true",
    "Godot runtime tonemap pass completion",
    "visual diff success",
    "GPU timestamp / performance claim",
]


# ---------------------------------------------------------------------------
# CPU reference — a verbatim port of generate_math_parity_evidence.py so the
# comparison here matches the tracked offline evidence bit for bit.
# ---------------------------------------------------------------------------

def f32(value: float) -> float:
    return struct.unpack("<f", struct.pack("<f", value))[0]


def input_texel(x: int, y: int, c: int) -> float:
    return f32(f32(float((x * 29 + y * 13 + c * 7) % 101)) / f32(50.0))


def linear_to_srgb_f32(value: float) -> float:
    v = f32(value)
    if v < 0.0:
        v = 0.0
    if v < f32(0.0031308):
        return f32(f32(12.92) * v)
    powed = f32(max(v, 0.0) ** f32(1.0 / 2.4))
    return f32(f32(f32(1.055) * powed) - f32(0.055))


def tonemap_texel(x: int, y: int, exposure: float, white: float, lum_mult: float) -> list[float]:
    out: list[float] = []
    for c in range(4):
        value = input_texel(x, y, c)
        if c < 3:
            scaled = f32(f32(value * f32(lum_mult)) * f32(exposure))
            out.append(linear_to_srgb_f32(scaled))
        else:
            out.append(value)  # alpha passthrough
    _ = white  # unused by TONEMAPPER_LINEAR
    return out


def build_input_bytes(width: int, height: int) -> bytes:
    flat: list[float] = []
    for y in range(height):
        for x in range(width):
            for c in range(4):
                flat.append(input_texel(x, y, c))
    return struct.pack(f"<{len(flat)}f", *flat)


def build_expected(width: int, height: int, exposure: float, white: float, lum_mult: float) -> list[float]:
    flat: list[float] = []
    for y in range(height):
        for x in range(width):
            flat.extend(tonemap_texel(x, y, exposure, white, lum_mult))
    return flat


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def now_iso() -> str:
    return _dt.datetime.now().astimezone().replace(microsecond=0).isoformat()


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def resolve_godot_exe(cli_exe: Path | None) -> Path:
    if cli_exe is not None:
        return cli_exe
    for key in ENV_GODOT_EXE:
        v = os.environ.get(key)
        if v:
            return Path(v)
    return DEFAULT_GODOT_EXE


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL", "") == "1"


def write_evidence(payload: dict[str, object]) -> None:
    EVIDENCE_OUT.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE_OUT.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[{SUBJECT}] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={payload.get('status')}")


def parse_env_marker(output: str) -> dict[str, object]:
    """Pull adapter / vendor / local_rd flags from RD_NATIVE_PROBE_ENV."""
    info: dict[str, object] = {"local_rendering_device": None, "adapter": None, "vendor": None}
    for line in output.splitlines():
        line = line.strip()
        if not line.startswith("RD_NATIVE_PROBE_ENV"):
            continue
        if "local_rd=null" in line:
            info["local_rendering_device"] = False
            return info
        if "local_rd=ok" in line:
            info["local_rendering_device"] = True
            a = line.find("adapter=")
            v = line.find(" vendor=")
            if a != -1 and v != -1:
                info["adapter"] = line[a + len("adapter="):v].strip()
                info["vendor"] = line[v + len(" vendor="):].strip()
        return info
    return info


NO_DEVICE_MARKERS = (
    "RD_NATIVE_PROBE_ENV local_rd=null",
    "Unable to create a rendering device",
    "Can't create a Direct3D 12 device",
    "Failed to create a D3D12",
    "No adapter found",
    "Your video card driver does not support",
)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--godot-exe", type=Path, default=None,
                    help="Godot console exe (default: RURIX_PROBE_GODOT_EXE / "
                         "RURIX_BENCH_GODOT_EXE env, else tracked template build)")
    args = ap.parse_args()

    started_at = now_iso()

    def base_evidence() -> dict[str, object]:
        return {
            "subject": SUBJECT,
            "slice": "route_b_s2_zero_patch_in_engine_probe",
            "generated_at": started_at,
            "run_url": github_run_url(),
            "tolerance": MAX_ABS_ERROR_TOLERANCE,
            "container": {
                "path": str(CONTAINER.relative_to(ROOT)) if CONTAINER.is_file() else str(CONTAINER),
                "sha256": sha256_file(CONTAINER),
            },
            "dxil": {
                "path": str(TONEMAP_DXIL.relative_to(ROOT)),
                "sha256": sha256_file(TONEMAP_DXIL),
            },
            "rts0": {
                "path": str(TONEMAP_RTS0.relative_to(ROOT)),
                "sha256": sha256_file(TONEMAP_RTS0),
            },
            "does_not_imply": DOES_NOT_IMPLY,
        }

    def finish(status: str, extra: dict[str, object]) -> int:
        payload = base_evidence()
        payload["status"] = status
        payload.update(extra)
        write_evidence(payload)
        if status == "success":
            print(f"[{SUBJECT}] PASS zero-patch in-engine RD-native compute; "
                  f"adapter={extra.get('adapter')} max_abs_diff={extra.get('max_abs_diff')}")
            return 0
        if status == "skip":
            print(f"[{SUBJECT}] SKIP {extra.get('reason')}")
            return 1 if require_real() else 0
        print(f"[{SUBJECT}] FAIL {extra.get('reason')}")
        return 1

    # --- resolve inputs ---------------------------------------------------
    godot_exe = resolve_godot_exe(args.godot_exe)
    if not godot_exe.is_file():
        return finish("skip", {"reason": f"Godot exe not found: {godot_exe}"})

    for path in (TONEMAP_DXIL, TONEMAP_RTS0):
        if not path.is_file():
            return finish("fail", {"reason": f"required artifact missing: {path.relative_to(ROOT)}"})

    # (Re)build the container from the tracked artifacts if it is missing so the
    # smoke is self-sufficient. When present it is used as-is (S1 fixture).
    container_source = "tracked_fixture"
    if not CONTAINER.is_file():
        gen = subprocess.run(
            [sys.executable, str(GENERATOR)],
            cwd=ROOT, capture_output=True, text=True,
        )
        if gen.returncode != 0 or not CONTAINER.is_file():
            return finish("fail", {
                "reason": "container generation failed",
                "generator_stdout": gen.stdout[-2000:],
                "generator_stderr": gen.stderr[-2000:],
            })
        container_source = "regenerated"

    # --- stage per-case input bins + expected outputs ---------------------
    WORK.mkdir(parents=True, exist_ok=True)
    manifest_cases: list[dict[str, object]] = []
    expected: dict[str, list[float]] = {}
    for case_id, w, h, exposure, white, lum in CASES:
        in_path = WORK / f"{case_id}.in.rgbaf32.bin"
        out_path = WORK / f"{case_id}.out.rgbaf32.bin"
        out_path.unlink(missing_ok=True)  # stale output can never be misread
        in_path.write_bytes(build_input_bytes(w, h))
        expected[case_id] = build_expected(w, h, exposure, white, lum)
        manifest_cases.append({
            "case_id": case_id,
            "width": w, "height": h,
            "exposure": exposure, "white": white, "luminance_multiplier": lum,
            "input_path": in_path.as_posix(),
            "output_path": out_path.as_posix(),
        })

    manifest = {
        "container_path": CONTAINER.resolve().as_posix(),
        "cases": manifest_cases,
    }
    manifest_path = WORK / "probe_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")

    # --- drive Godot ------------------------------------------------------
    command = [
        str(godot_exe),
        "--path", str(PROBE_PROJECT),
        "--rendering-driver", "d3d12",
        "--rendering-method", "forward_plus",
        "--",
        "--manifest", manifest_path.as_posix(),
    ]
    try:
        proc = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True, timeout=TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        return finish("fail", {"reason": f"probe timed out after {TIMEOUT_SECONDS}s", "command": command})

    output = (proc.stdout or "") + "\n" + (proc.stderr or "")
    output = output.replace("\r\n", "\n")
    env_info = parse_env_marker(output)
    log_path = WORK / "probe_stdout.log"
    log_path.write_text(output, encoding="utf-8", newline="\n")

    result_ok = "RD_NATIVE_PROBE_RESULT status=ok" in output
    device_present = env_info.get("local_rendering_device") is True

    common = {
        "godot_exe": str(godot_exe),
        "godot_exe_sha256": sha256_file(godot_exe),
        "container_source": container_source,
        "adapter": env_info.get("adapter"),
        "vendor": env_info.get("vendor"),
        "local_rendering_device": env_info.get("local_rendering_device"),
        "exit_code": proc.returncode,
        "probe_log": str(log_path.relative_to(ROOT)),
    }

    # No usable device -> honest SKIP (unless a real run is required).
    if not device_present:
        if any(marker in output for marker in NO_DEVICE_MARKERS) or not result_ok:
            reason = "no usable D3D12 device (local rendering device unavailable)"
            if not any(marker in output for marker in NO_DEVICE_MARKERS) and env_info.get("local_rendering_device") is None:
                reason = "probe did not report a rendering device; treating as no-device"
            return finish("skip", {**common, "reason": reason, "stdout_tail": output[-3000:]})

    if proc.returncode != 0 or not result_ok:
        return finish("fail", {
            **common,
            "reason": "probe reported failure or did not complete",
            "stdout_tail": output[-3000:],
        })

    # --- compare GPU readback against the CPU reference -------------------
    per_case: list[dict[str, object]] = []
    overall_max = 0.0
    for entry in manifest_cases:
        case_id = str(entry["case_id"])
        w = int(entry["width"]); h = int(entry["height"])
        out_path = Path(str(entry["output_path"]))
        exp = expected[case_id]
        if not out_path.is_file():
            return finish("fail", {**common, "reason": f"missing readback output for {case_id}"})
        raw = out_path.read_bytes()
        want_bytes = w * h * 16
        if len(raw) != want_bytes:
            return finish("fail", {
                **common,
                "reason": f"{case_id}: readback {len(raw)} bytes != expected {want_bytes}",
            })
        gpu = struct.unpack(f"<{w * h * 4}f", raw)
        case_max = 0.0
        worst = None
        for i, (g, e) in enumerate(zip(gpu, exp)):
            d = abs(g - e)
            if d > case_max:
                case_max = d
                worst = {"index": i, "gpu": g, "cpu": e}
        overall_max = max(overall_max, case_max)
        per_case.append({
            "case_id": case_id,
            "width": w, "height": h,
            "constants": {
                "exposure": entry["exposure"], "white": entry["white"],
                "luminance_multiplier": entry["luminance_multiplier"],
            },
            "output_bytes": len(raw),
            "output_sha256": sha256_bytes(raw),
            "max_abs_diff": case_max,
            "within_tolerance": case_max <= MAX_ABS_ERROR_TOLERANCE,
            "worst_texel": worst,
        })

    all_within = all(bool(c["within_tolerance"]) for c in per_case)
    if not all_within:
        return finish("fail", {
            **common,
            "reason": f"per-texel comparison exceeded tolerance {MAX_ABS_ERROR_TOLERANCE}",
            "max_abs_diff": overall_max,
            "cases": per_case,
        })

    return finish("success", {
        **common,
        "q1_result": (
            "create_local_rendering_device() returns a valid RenderingDevice under "
            "--rendering-driver d3d12; the zero-patch local-RD route is viable "
            "(main-RD + call_on_render_thread fallback not needed)"
        ),
        "pipeline_path": [
            "shader_create_from_bytecode (-> shader_create_from_container)",
            "compute_pipeline_create (-> CreateRootSignature + PSO from container bytes)",
            "uniform_set_create([t0 SRV, u0 UAV], shader, 0)",
            "compute_list dispatch + submit + sync",
            "texture_get_data readback",
        ],
        "max_abs_diff": overall_max,
        "cases": per_case,
    })


if __name__ == "__main__":
    sys.exit(main())
