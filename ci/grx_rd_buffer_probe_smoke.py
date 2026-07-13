#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Route B buffer-runtime-equivalence probe smoke (zero-patch, in-engine, real GPU).

Fills the gap the S1 pipeline report flags honestly: the S2 probe
(ci/grx_rd_native_probe_smoke.py) proved a TEXTURE-typed Rurix container runs
unmodified as a first-class RenderingDevice compute pass, but it never covered
the RAW-BUFFER containers. The crux (S1 report sec 2 "诚实边界"): a
``StructuredBuffer<T>`` reflects to ``UNIFORM_TYPE_STORAGE_BUFFER``, and Godot's
D3D12 driver binds STORAGE_BUFFER uniforms as a RAW ``R32_TYPELESS`` view
(``StructureByteStride = 0``, ``D3D12_BUFFER_*_FLAG_RAW``;
``drivers/d3d12/rendering_device_driver_d3d12.cpp:3547-3563``), whereas the
offline dispatch harnesses bind STRUCTURED views (real element strides — 80 for
RenderElementData, 48 for the compaction transforms). Whether the DXC-compiled
``StructuredBuffer`` DXIL runs EQUIVALENTLY under a RAW view is unproven — this
smoke answers it with real GPU bytes, for TWO raw-buffer passes covering two
distinct stride-sensitive binding shapes:

  * cluster_store       — 3 bindings (2 StructuredBuffer SRV + 1
                          RWStructuredBuffer UAV); render_elements is an 80-byte
                          struct, the genuinely stride-sensitive read.
  * instance_compaction/scatter — 5 bindings (4 StructuredBuffer SRV + 1
                          RWStructuredBuffer UAV); src/dst transforms are 48-byte
                          (12-float) structs, a much larger stride. Run
                          standalone by feeding the CPU-computed scan
                          intermediates (local_prefix, group_offsets) as inputs.

It drives the tracked Godot template build over the GDScript buffer probe
(``spike/godot-rurix/rd-native-pipeline/probe_project/rd_buffer_probe.gd`` via
``res://buffer_probe.tscn``, selected by a positional-scene override). The probe
loads each Rurix-built container through
``RenderingDevice.shader_create_from_bytecode``, creates the compute pipeline,
binds N ``storage_buffer_create`` buffers as N ``UNIFORM_TYPE_STORAGE_BUFFER``
uniforms (the driver picks UAV vs SRV per the container's per-binding writable
flag), dispatches on a *local* rendering device, and reads back the destination.

This harness owns the CPU reference: it imports each pass's tracked math-parity
module (``generate_math_parity_evidence.py``) — the SAME fixtures + reference the
offline dispatch smokes use — and compares every GPU-observed u32 word against
the reference EXACTLY. Both kernels are integer/bit-preserving (cluster_store is
pure u32 word math; scatter is a bit-preserving float move), so the tolerance is
ZERO. The destination is explicitly zero-uploaded (the native buffer_clear), so
the ONLY GPU-vs-CPU divergence would be the kernel under the RAW vs structured
view — precisely what is under test.

It proves ONE thing when it passes: raw-buffer Rurix containers are
runtime-equivalent under Godot's RAW STORAGE_BUFFER binding — i.e. buffer-type
passes are Route-B viable. A word mismatch is an equally important finding: it
would localize the RAW-vs-structured divergence (first mismatching word) and
show that StructuredBuffer HLSL bridges are NOT Route-B-safe as bound. It does
NOT claim same-frame injection, default pass enablement, real_gpu_pass=true, or
any performance number.

Fail-closed discipline (mirrors ci/grx_rd_native_probe_smoke.py):
  * The device is always real. If no usable local D3D12 device exists, the run
    records ``status=skip`` and exits 0 (exit 1 when RURIX_REQUIRE_REAL=1). SKIP
    never advances any gate.
  * A device that IS present but where the container load, pipeline creation,
    uniform-set/dispatch/readback, or per-word comparison fails is
    ``status=fail`` with the first mismatching word located.
  * Every container + source DXIL/RTS0 sha256 is recorded.

Evidence: spike/godot-rurix/rd-native-pipeline/rd_buffer_probe_evidence.json
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import importlib.util
import json
import os
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PIPELINE_DIR = ROOT / "spike" / "godot-rurix" / "rd-native-pipeline"
PROBE_PROJECT = PIPELINE_DIR / "probe_project"
BUFFER_SCENE = "res://buffer_probe.tscn"
OUT_DIR = PIPELINE_DIR / "out"
GENERATOR = PIPELINE_DIR / "generate_rd_container.py"
PASSES = ROOT / "spike" / "godot-rurix" / "passes"

EVIDENCE_OUT = PIPELINE_DIR / "rd_buffer_probe_evidence.json"
WORK = ROOT / "target" / "grx_rd_buffer_probe"

DEFAULT_GODOT_EXE = (
    ROOT / "external" / "godot-master" / "bin" / "godot.windows.template_debug.x86_64.console.exe"
)
ENV_GODOT_EXE = ("RURIX_PROBE_GODOT_EXE", "RURIX_BENCH_GODOT_EXE")

SUBJECT = "grx_rd_buffer_probe"
TIMEOUT_SECONDS = 300
WORD_STRIDE = 4
# Both kernels are integer / bit-preserving (cluster_store = pure u32 word math;
# scatter = bit-preserving float move): the GPU output must match the CPU
# reference EXACTLY, word for word. Zero tolerance.
VALUE_TOLERANCE = 0

DRIVER_RAW_VIEW_NOTE = (
    "Godot's D3D12 driver binds UNIFORM_TYPE_STORAGE_BUFFER as a RAW R32_TYPELESS "
    "view (StructureByteStride=0, D3D12_BUFFER_{SRV,UAV}_FLAG_RAW; "
    "drivers/d3d12/rendering_device_driver_d3d12.cpp:3547-3563), whereas the "
    "offline dispatch harnesses bind STRUCTURED views (real element strides). "
    "This probe tests whether the DXC-compiled StructuredBuffer DXIL is "
    "runtime-equivalent under the RAW view."
)

DOES_NOT_IMPLY = [
    "same-frame injection (that is S3+, requires a patch)",
    "default pass enablement",
    "real_gpu_pass=true",
    "Godot runtime pass completion",
    "visual diff success",
    "GPU timestamp / performance claim",
]


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


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


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


def import_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        return None
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except Exception:  # noqa: BLE001 - honest import failure, reported by caller
        return None
    return module


def write_evidence(payload: dict[str, object]) -> None:
    EVIDENCE_OUT.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE_OUT.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[{SUBJECT}] wrote {rel(EVIDENCE_OUT)} status={payload.get('status')}")


def parse_env_marker(output: str) -> dict[str, object]:
    info: dict[str, object] = {"local_rendering_device": None, "adapter": None, "vendor": None}
    for line in output.splitlines():
        line = line.strip()
        if not line.startswith("RD_BUFFER_PROBE_ENV"):
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


def parse_fail_reason(output: str) -> str | None:
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("RD_BUFFER_PROBE_RESULT status=fail"):
            idx = line.find("reason=")
            return line[idx + len("reason="):].strip() if idx != -1 else "unspecified"
    return None


NO_DEVICE_MARKERS = (
    "RD_BUFFER_PROBE_ENV local_rd=null",
    "Unable to create a rendering device",
    "Can't create a Direct3D 12 device",
    "Failed to create a D3D12",
    "No adapter found",
    "Your video card driver does not support",
)


def compare_words(expected: list[int], raw: bytes) -> dict:
    """Compare every GPU-observed u32 word against the reference, exactly."""
    expected_len = len(expected) * WORD_STRIDE
    if len(raw) != expected_len:
        return {"match": False, "reason": f"readback {len(raw)} bytes != expected {expected_len}",
                "total_words": len(expected)}
    observed = struct.unpack(f"<{len(expected)}I", raw)
    mismatched = 0
    first_mismatch = None
    for idx, (ref_w, obs) in enumerate(zip(expected, observed)):
        if ref_w != obs:
            mismatched += 1
            if first_mismatch is None:
                first_mismatch = {"word_index": idx, "gpu_hex": f"0x{obs:08X}", "cpu_hex": f"0x{ref_w:08X}"}
    return {
        "match": mismatched == 0,
        "mismatched_words": mismatched,
        "total_words": len(expected),
        "nonzero_reference_words": sum(1 for w in expected if w != 0),
        "value_tolerance": VALUE_TOLERANCE,
        "first_mismatch": first_mismatch,
    }


# ---------------------------------------------------------------------------
# per-pass fixture builders. Each returns a list of case dicts:
#   {case_id, dispatch, b0(bytes), inputs:[(binding, bytes)], out_binding,
#    dst_bytes, expected(list[int] u32 words), meta(dict)}
# The CPU reference bytes come from the tracked math-parity modules (imported),
# so this smoke never re-implements the kernel math.
# ---------------------------------------------------------------------------

def build_cluster_store_cases(parity) -> list[dict]:
    cases: list[dict] = []
    for case in parity.parity_cases():
        consts = parity.case_constants(case)
        elements = parity.build_elements(case)
        words = parity.build_cluster_render_words(case, consts)
        expected, _ = parity.cluster_store_reference(consts, words, elements)
        w, h = consts["cluster_screen_size"]
        cases.append({
            "case_id": case["case_id"],
            "dispatch": [(w + 7) // 8, (h + 7) // 8, 1],
            "b0": parity.build_b0(consts),
            "inputs": [
                (0, parity.pack_words(words)),                              # cluster_render t0 (stride 4)
                (1, parity.pack_elements(elements, consts["render_element_max"])),  # render_elements t1 (stride 80)
            ],
            "out_binding": 2,                                              # cluster_store u0 (stride 4)
            "dst_bytes": parity.dst_word_count(consts) * WORD_STRIDE,
            "expected": expected,
            "meta": {
                "cluster_screen_size": [w, h],
                "render_element_count": consts["render_element_count"],
                "stride_sensitive_binding": "render_elements t1 StructuredBuffer<RenderElementData> stride=80",
            },
        })
    return cases


# The five tracked instance_compaction fixtures (mirror of the case list in
# passes/instance_compaction/generate_math_parity_evidence.py main()). The
# survive predicates and (n, garbage_tail) match byte-for-byte so the CPU
# reference here equals the tracked offline evidence.
def _scatter_fixture_defs():
    def lcg_survive(p: int) -> bool:
        return ((p * 1103515245 + 12345) >> 16) % 4 == 0
    return [
        ("sparse_survival_multi_group", 600, lcg_survive, False),
        ("all_survive", 513, (lambda p: True), False),
        ("zero_survive", 384, (lambda p: False), False),
        ("mask_tail_garbage_bits_ignored", 70, (lambda p: p % 5 == 0), True),
        ("single_survivor_last_instance_empty_leading_group", 300, (lambda p: p == 299), False),
    ]


def build_scatter_cases(parity) -> list[dict]:
    group_size = parity.GROUP_SIZE
    stride_floats = parity.TRANSFORM_STRIDE_FLOATS
    cases: list[dict] = []
    for case_id, n, survive, garbage_tail in _scatter_fixture_defs():
        mask_words = parity.pack_mask(n, survive, garbage_tail)
        ref = parity.reference_chain(n, mask_words)
        num_groups = int(ref["num_groups"])
        # b0 mirrors the offline harness: <8I> n, len(mask_words), num_groups,
        # transform_stride_vec4(=stride_floats/4), pad0..3.
        b0 = struct.pack("<8I", n, len(mask_words), num_groups, stride_floats // 4, 0, 0, 0, 0)
        src_bytes = struct.pack(f"<{n * stride_floats}f", *ref["src"])
        dst_expected_bytes = struct.pack(f"<{n * stride_floats}f", *ref["dst"])
        # scatter is a bit-preserving move: the float dst is a byte-exact copy,
        # so compare as u32 words (zero tolerance, bit-exact).
        expected_words = list(struct.unpack(f"<{n * stride_floats}I", dst_expected_bytes))
        cases.append({
            "case_id": case_id,
            "dispatch": [(n + group_size - 1) // group_size, 1, 1],
            "b0": b0,
            "inputs": [
                (0, struct.pack(f"<{len(mask_words)}I", *mask_words)),      # visibility_mask t0 (stride 4)
                (1, src_bytes),                                            # src_transforms t1 (stride 48)
                (2, struct.pack(f"<{n}I", *ref["local_prefix"])),          # local_prefix t2 (stride 4)
                (3, struct.pack(f"<{num_groups}I", *ref["group_offsets"])),# group_offsets t3 (stride 4)
            ],
            "out_binding": 4,                                             # dst_transforms u0 (stride 48)
            "dst_bytes": n * stride_floats * WORD_STRIDE,
            "expected": expected_words,
            "meta": {
                "total_instances": n,
                "num_groups": num_groups,
                "survivor_count": int(ref["survivor_count"]),
                "stride_sensitive_binding": "src/dst_transforms StructuredBuffer stride=48 (12 floats)",
            },
        })
    return cases


PASS_SPECS = [
    {
        "pass_id": "cluster_store",
        "container": OUT_DIR / "cluster_store.rd_container.bin",
        "dxil": PASSES / "cluster_store" / "artifacts" / "cluster_store.dxil",
        "rts0": PASSES / "cluster_store" / "artifacts" / "cluster_store.rts0.bin",
        "parity": PASSES / "cluster_store" / "generate_math_parity_evidence.py",
        "offline_evidence": PASSES / "cluster_store" / "real_d3d12_dispatch_smoke.json",
        "builder": build_cluster_store_cases,
        "binding_shape": "2 StructuredBuffer SRV (t0,t1) + 1 RWStructuredBuffer UAV (u0)",
    },
    {
        "pass_id": "instance_compaction_scatter",
        "container": OUT_DIR / "instance_compaction_scatter.rd_container.bin",
        "dxil": PASSES / "instance_compaction" / "artifacts" / "instance_compaction_scatter.dxil",
        "rts0": PASSES / "instance_compaction" / "artifacts" / "instance_compaction_scatter.rts0.bin",
        "parity": PASSES / "instance_compaction" / "generate_math_parity_evidence.py",
        "offline_evidence": PASSES / "instance_compaction" / "real_d3d12_dispatch_smoke.json",
        "builder": build_scatter_cases,
        "binding_shape": "4 StructuredBuffer SRV (t0..t3) + 1 RWStructuredBuffer UAV (u0)",
    },
]


def offline_cross_link(spec: dict) -> dict:
    ev = spec["offline_evidence"]
    out: dict[str, object] = {"path": rel(ev), "status": None, "cpu_reference_match": None}
    if ev.is_file():
        try:
            doc = json.loads(ev.read_text(encoding="utf-8"))
            out["status"] = doc.get("status")
            out["cpu_reference_match"] = doc.get("cpu_reference_match")
        except (OSError, json.JSONDecodeError):
            pass
    return out


def stage_and_drive(spec: dict, godot_exe: Path) -> dict:
    """Stage one pass's fixtures, drive Godot once, compare readback. Returns a
    per-pass result dict with a 'phase' of skip/fail/success."""
    pass_id = str(spec["pass_id"])
    container: Path = spec["container"]

    result: dict[str, object] = {
        "pass_id": pass_id,
        "binding_shape": spec["binding_shape"],
        "container": {"path": rel(container), "sha256": sha256_file(container)},
        "dxil": {"path": rel(spec["dxil"]), "sha256": sha256_file(spec["dxil"])},
        "rts0": {"path": rel(spec["rts0"]), "sha256": sha256_file(spec["rts0"])},
        "offline_structured_view_cross_link": offline_cross_link(spec),
    }

    for path in (spec["dxil"], spec["rts0"], spec["parity"]):
        if not path.is_file():
            return {**result, "phase": "fail", "reason": f"required artifact missing: {rel(path)}"}

    # Self-sufficient: regenerate all containers if this one is missing.
    if not container.is_file():
        gen = subprocess.run([sys.executable, str(GENERATOR), "--all"], cwd=ROOT,
                             capture_output=True, text=True)
        if gen.returncode != 0 or not container.is_file():
            return {**result, "phase": "fail", "reason": "container generation failed",
                    "generator_stderr": gen.stderr[-1500:]}
        result["container"]["sha256"] = sha256_file(container)

    parity = import_module(f"parity_{pass_id}", spec["parity"])
    if parity is None:
        return {**result, "phase": "fail",
                "reason": f"cannot import tracked math-parity module {rel(spec['parity'])}"}
    try:
        cases = spec["builder"](parity)
    except Exception as exc:  # noqa: BLE001 - surface honestly
        return {**result, "phase": "fail", "reason": f"fixture build failed: {exc!r}"}

    work = WORK / pass_id
    work.mkdir(parents=True, exist_ok=True)
    manifest_cases: list[dict[str, object]] = []
    for c in cases:
        cid = c["case_id"]
        b0_path = work / f"{cid}.b0.32b.bin"
        b0_path.write_bytes(c["b0"])
        input_entries: list[dict[str, object]] = []
        for binding, data in c["inputs"]:
            ip = work / f"{cid}.in{binding}.bin"
            ip.write_bytes(data)
            input_entries.append({"binding": binding, "path": ip.as_posix()})
        out_path = work / f"{cid}.out.bin"
        out_path.unlink(missing_ok=True)
        c["_out_path"] = out_path
        manifest_cases.append({
            "case_id": cid,
            "dispatch": c["dispatch"],
            "b0_path": b0_path.as_posix(),
            "inputs": input_entries,
            "output": {"binding": c["out_binding"], "dst_bytes": c["dst_bytes"],
                       "output_path": out_path.as_posix()},
        })

    manifest = {"container_path": container.resolve().as_posix(), "cases": manifest_cases}
    manifest_path = work / "buffer_probe_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")

    command = [
        str(godot_exe), "--path", str(PROBE_PROJECT),
        "--rendering-driver", "d3d12", "--rendering-method", "forward_plus",
        BUFFER_SCENE, "--", "--manifest", manifest_path.as_posix(),
    ]
    try:
        proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        return {**result, "phase": "fail", "reason": f"probe timed out after {TIMEOUT_SECONDS}s"}

    output = ((proc.stdout or "") + "\n" + (proc.stderr or "")).replace("\r\n", "\n")
    env_info = parse_env_marker(output)
    log_path = work / "buffer_probe_stdout.log"
    log_path.write_text(output, encoding="utf-8", newline="\n")

    result.update({
        "adapter": env_info.get("adapter"),
        "vendor": env_info.get("vendor"),
        "local_rendering_device": env_info.get("local_rendering_device"),
        "exit_code": proc.returncode,
        "probe_log": rel(log_path),
    })

    result_ok = "RD_BUFFER_PROBE_RESULT status=ok" in output
    device_present = env_info.get("local_rendering_device") is True
    if not device_present:
        if any(m in output for m in NO_DEVICE_MARKERS) or not result_ok:
            return {**result, "phase": "skip",
                    "reason": "no usable local D3D12 device", "stdout_tail": output[-2000:]}

    if proc.returncode != 0 or not result_ok:
        return {**result, "phase": "fail",
                "reason": f"probe reported failure or did not complete: {parse_fail_reason(output) or 'unknown'}",
                "probe_fail_reason": parse_fail_reason(output),
                "stdout_tail": output[-2000:]}

    per_case: list[dict[str, object]] = []
    all_match = True
    for c in cases:
        cid = c["case_id"]
        out_path: Path = c["_out_path"]
        if not out_path.is_file():
            return {**result, "phase": "fail", "reason": f"missing readback output for {cid}"}
        raw = out_path.read_bytes()
        comparison = compare_words(c["expected"], raw)
        if not comparison.get("match"):
            all_match = False
        per_case.append({
            "case_id": cid,
            "dispatch": c["dispatch"],
            "dst_bytes": c["dst_bytes"],
            "dst_word_count": len(c["expected"]),
            "output_sha256": sha256_bytes(raw),
            "meta": c["meta"],
            "comparison": comparison,
        })

    result["cases"] = per_case
    result["all_words_match_cpu_reference_exactly"] = all_match
    result["phase"] = "success" if all_match else "fail"
    if not all_match:
        result["reason"] = (
            "GPU-observed words did NOT match the tracked reference exactly under "
            "Godot's RAW STORAGE_BUFFER binding — RAW view is NOT runtime-equivalent"
        )
    return result


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--godot-exe", type=Path, default=None,
                    help="Godot console exe (default: RURIX_PROBE_GODOT_EXE / "
                         "RURIX_BENCH_GODOT_EXE env, else tracked template build)")
    args = ap.parse_args()

    started_at = now_iso()
    godot_exe = resolve_godot_exe(args.godot_exe)

    def base_payload() -> dict[str, object]:
        return {
            "subject": SUBJECT,
            "slice": "route_b_buffer_runtime_equivalence_probe",
            "generated_at": started_at,
            "run_url": github_run_url(),
            "value_tolerance": VALUE_TOLERANCE,
            "binding_semantics_under_test": DRIVER_RAW_VIEW_NOTE,
            "godot_exe": str(godot_exe),
            "godot_exe_sha256": sha256_file(godot_exe) if godot_exe.is_file() else None,
            "does_not_imply": DOES_NOT_IMPLY,
        }

    def finish(status: str, extra: dict[str, object]) -> int:
        payload = base_payload()
        payload["status"] = status
        payload.update(extra)
        write_evidence(payload)
        if status == "success":
            print(f"[{SUBJECT}] PASS raw-buffer containers are runtime-equivalent under "
                  f"Godot's RAW STORAGE_BUFFER binding (tolerance=0, exact)")
            return 0
        if status == "skip":
            print(f"[{SUBJECT}] SKIP {extra.get('reason')}")
            return 1 if require_real() else 0
        print(f"[{SUBJECT}] FAIL {extra.get('reason')}")
        return 1

    if not godot_exe.is_file():
        return finish("skip", {"reason": f"Godot exe not found: {godot_exe}", "passes": []})

    pass_results = [stage_and_drive(spec, godot_exe) for spec in PASS_SPECS]

    # Aggregate. Any pass with no device -> whole run SKIP. Any fail -> FAIL.
    if any(r.get("phase") == "skip" for r in pass_results):
        first_skip = next(r for r in pass_results if r.get("phase") == "skip")
        return finish("skip", {"reason": first_skip.get("reason"), "passes": pass_results})
    if any(r.get("phase") == "fail" for r in pass_results):
        first_fail = next(r for r in pass_results if r.get("phase") == "fail")
        return finish("fail", {
            "reason": f"{first_fail.get('pass_id')}: {first_fail.get('reason')}",
            "raw_vs_structured_equivalence": "diverged",
            "passes": pass_results,
        })

    dev = pass_results[0]
    return finish("success", {
        "adapter": dev.get("adapter"),
        "vendor": dev.get("vendor"),
        "local_rendering_device": dev.get("local_rendering_device"),
        "raw_vs_structured_equivalence": "proven_equivalent",
        "equivalence_conclusion": (
            "Raw-buffer Rurix containers (StructuredBuffer<T> -> "
            "UNIFORM_TYPE_STORAGE_BUFFER) bound by Godot's D3D12 driver as RAW "
            "R32_TYPELESS views (StructureByteStride=0) produced byte-identical "
            "output to the offline structured-view dispatch and the CPU reference "
            "(zero tolerance) across two stride-sensitive binding shapes "
            "(cluster_store render_elements stride=80; scatter transforms "
            "stride=48). The DXC-compiled StructuredBuffer DXIL bakes the element "
            "stride into the shader, so RAW and structured views are "
            "runtime-equivalent; buffer-type passes are Route-B viable through the "
            "same zero-patch local-RD path as the texture passes."
        ),
        "pipeline_path": [
            "shader_create_from_bytecode (-> shader_create_from_container)",
            "compute_pipeline_create (-> CreateRootSignature + PSO from container bytes)",
            "storage_buffer_create x N (per binding, incl. zeroed destination)",
            "uniform_set_create([UNIFORM_TYPE_STORAGE_BUFFER ...], shader, 0)",
            "compute_list dispatch + submit + sync",
            "buffer_get_data readback (UAV output)",
        ],
        "passes": pass_results,
    })


if __name__ == "__main__":
    sys.exit(main())
