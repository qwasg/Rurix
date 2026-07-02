#!/usr/bin/env python3
"""Probe the local Godot/Rurix toolchain without mutating the machine."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass

from godot_rurix_patch_stack import evaluate_patch_stack


ROOT = pathlib.Path(__file__).resolve().parents[1]
EXTERNAL_GODOT = ROOT / "external" / "godot-master"
SCONSTRUCT = EXTERNAL_GODOT / "SConstruct"
RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
LOCAL_LOG_DIR = ROOT / "target" / "grx"
JSON_REPORT = LOCAL_LOG_DIR / "godot_toolchain_probe.json"
BUILD_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_scons_build_summary.json"
LOAD_SMOKE_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_load_smoke_summary.json"
BENCH_SMOKE_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_bench_project_smoke_summary.json"
BENCH_RUNNER_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_bench_runner_summary.json"
BENCH_DIR = ROOT / "spike" / "godot-rurix" / "bench"
GRX006_SCHEMA_SAMPLE_FILES = (
    BENCH_DIR / "schemas" / "baseline_evidence.schema.json",
    BENCH_DIR / "schemas" / "perf_gate_input.schema.json",
    BENCH_DIR / "samples" / "baseline_smoke_example.json",
    BENCH_DIR / "samples" / "perf_gate_failing_example.json",
)
GRX006_PERF_GATE_SCRIPT = BENCH_DIR / "perf_gate.py"
GRX006_BASELINE_SMOKE_SAMPLE = BENCH_DIR / "samples" / "baseline_smoke_example.json"
GRX006_FORBIDDEN_SKIP_SAMPLE = (
    BENCH_DIR / "samples" / "perf_gate_forbidden_skip_example.json"
)
GRX006_MISSING_SAMPLE_COUNT_SAMPLE = (
    BENCH_DIR / "samples" / "baseline_missing_sample_count_example.json"
)
GRX007_VISUAL_DIFF_SCRIPT = BENCH_DIR / "visual_diff.py"
GRX007_VISUAL_SCHEMA = BENCH_DIR / "schemas" / "visual_diff_evidence.schema.json"
GRX007_VISUAL_PLACEHOLDER_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_placeholder.json"
)
GRX007_VISUAL_LDR_PASS_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_ldr_pass_example.json"
)
GRX007_VISUAL_MISSING_LDR_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_pass_missing_ldr_example.json"
)
GRX007_VISUAL_MISMATCH_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_mismatch_example.json"
)
GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_skip_with_fake_ldr_example.json"
)
GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_pass_missing_frame_artifact_example.json"
)
GRX008_FALLBACK_TELEMETRY_SCRIPT = BENCH_DIR / "fallback_telemetry.py"
GRX008_FALLBACK_SCHEMA = BENCH_DIR / "schemas" / "fallback_telemetry.schema.json"
GRX008_FALLBACK_PLACEHOLDER_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_placeholder.json"
)
GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_full_null_timestamp_example.json"
)
GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_scaffold_fallback_inactive_example.json"
)
GRX009_PASS_DIR = BENCH_DIR.parent / "passes" / "luminance_reduction"
GRX009_PASS_CONTRACT = GRX009_PASS_DIR / "PASS_CONTRACT.md"
GRX009_PASS_MANIFEST = GRX009_PASS_DIR / "pass_manifest.json"
GRX009_PATCH_0002 = (
    BENCH_DIR.parent / "patches" / "0002-rurix-accel-luminance-pass-gate.patch"
)
GRX009_PATCH_0001 = (
    BENCH_DIR.parent / "patches" / "0001-rurix-accel-module-scaffold.patch"
)
GRX009_PATCH_0003 = (
    BENCH_DIR.parent
    / "patches"
    / "0003-rurix-accel-luminance-core-callsite-wiring.patch"
)
GRX009_BRIDGE_LIB = ROOT / "src" / "rurix-godot" / "src" / "lib.rs"
GRX009_DISABLED_TELEMETRY_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_luminance_disabled_example.json"
)
GRX009_CALLSITE_WIRED_TELEMETRY_SAMPLE = (
    BENCH_DIR
    / "samples"
    / "fallback_telemetry_luminance_callsite_wired_disabled_example.json"
)
GRX009_COMPILE_EVIDENCE = GRX009_PASS_DIR / "offline_compile_evidence.json"
GRX009_COMPILE_SCHEMA = GRX009_PASS_DIR / "compile_evidence.schema.json"
LOCAL_SCONS_VENV = LOCAL_LOG_DIR / "scons-venv"
LOCAL_SCONS_PYTHON = LOCAL_SCONS_VENV / "Scripts" / "python.exe"
LOCAL_GODOT_LOCALAPPDATA = LOCAL_LOG_DIR / "localappdata"
LOCAL_GODOT_BUILD_DEPS = LOCAL_GODOT_LOCALAPPDATA / "Godot" / "build_deps"
GODOT_INSTALL_ACCESSKIT = (
    ROOT / "external" / "godot-master" / "misc" / "scripts" / "install_accesskit.py"
)
GODOT_INSTALL_D3D12_DEPS = (
    ROOT / "external" / "godot-master" / "misc" / "scripts" / "install_d3d12_sdk_windows.py"
)
VSWHERE = pathlib.Path(
    os.environ.get(
        "RURIX_VSWHERE",
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    )
)
VCVARSALL_REL = pathlib.Path("VC") / "Auxiliary" / "Build" / "vcvarsall.bat"

DEFAULT_SDK_INCLUDE_ROOTS = [
    pathlib.Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
    / "Windows Kits"
    / "10"
    / "Include",
]
DEFAULT_SDK_BIN_ROOTS = [
    pathlib.Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
    / "Windows Kits"
    / "10"
    / "bin",
]
HEADER_CANDIDATES = ("d3d12.h", "dxgi1_6.h")
TOOL_CANDIDATES = ("dxc.exe", "dxv.exe")
SCONS_BUILD_ARGS = (
    "platform=windows target=template_debug d3d12=yes "
    "module_rurix_accel_enabled=yes disable_path_overrides=no"
)
SCONS_ICE_ARGS = (
    SCONS_BUILD_ARGS + " num_jobs=1 verbose=yes angle=no silence_msvc=no"
)
PROBE_COMMAND = "py -3 ci/godot_rurix_toolchain_probe.py"
LOAD_SMOKE_COMMAND = r"py -3 ci\godot_rurix_load_smoke.py"
TOOLCHAIN_ENV_KEYS = (
    "VSINSTALLDIR",
    "VCINSTALLDIR",
    "VCToolsInstallDir",
    "VCTOOLSINSTALLDIR",
    "VisualStudioVersion",
)
REQUIRED_BUILD_ARTIFACT_KEYS = (
    "godot_exe",
    "godot_console_exe",
    "module_rurix_accel_lib",
)
REQUIRED_SCONS_ARGS = ("disable_path_overrides=no",)
VS_INSTALL_RE = re.compile(
    r"(?i)([A-Z]:\\[^:\n\r]*?Microsoft Visual Studio\\\d{4}\\[^\\\n\r]+)"
)

HOST_MACHINE = platform.machine().lower()
if HOST_MACHINE in ("amd64", "x86_64", "x64"):
    GODOT_WINDOWS_ARCH = "x86_64"
    ACCESSKIT_WINDOWS_ARCH = "x86_64"
    PIX_WINDOWS_ARCH = "x64"
    AGILITY_WINDOWS_ARCH = "x64"
elif HOST_MACHINE in ("arm64", "aarch64"):
    GODOT_WINDOWS_ARCH = "arm64"
    ACCESSKIT_WINDOWS_ARCH = "arm64"
    PIX_WINDOWS_ARCH = "ARM64"
    AGILITY_WINDOWS_ARCH = "arm64"
elif HOST_MACHINE in ("x86", "i386", "i686"):
    GODOT_WINDOWS_ARCH = "x86_32"
    ACCESSKIT_WINDOWS_ARCH = "x86"
    PIX_WINDOWS_ARCH = "x86"
    AGILITY_WINDOWS_ARCH = "x86"
else:
    GODOT_WINDOWS_ARCH = "x86_64"
    ACCESSKIT_WINDOWS_ARCH = "x86_64"
    PIX_WINDOWS_ARCH = "x64"
    AGILITY_WINDOWS_ARCH = "x64"


@dataclass
class ProbeResult:
    name: str
    status: str
    reason: str
    details: dict[str, object]


def print_result(result: ProbeResult) -> None:
    print(f"[godot-toolchain] {result.name}: {result.status} - {result.reason}")
    if result.details:
        for key, value in sorted(result.details.items()):
            print(f"[godot-toolchain]   {key}: {value}")


def completed_output(proc: subprocess.CompletedProcess[str]) -> str:
    parts = []
    stdout = (proc.stdout or "").strip()
    stderr = (proc.stderr or "").strip()
    if stdout:
        parts.append(stdout)
    if stderr:
        parts.append(stderr)
    return " | ".join(parts)


def cleaned_lines(text: str) -> list[str]:
    return [line.strip() for line in text.splitlines() if line.strip()]


def normalize_string(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    stripped = value.strip()
    return stripped or None


def infer_vs_installation_root(raw_path: object) -> str | None:
    candidate = normalize_string(raw_path)
    if not candidate:
        return None
    match = VS_INSTALL_RE.search(candidate)
    if match:
        return match.group(1)
    return None


def run_cmd_chain(
    command: str,
    *,
    vcvarsall: str | None = None,
) -> subprocess.CompletedProcess[str]:
    if vcvarsall:
        with tempfile.NamedTemporaryFile(
            "w",
            suffix=".bat",
            delete=False,
            encoding="utf-8",
            newline="\r\n",
        ) as handle:
            handle.write("@echo off\r\n")
            handle.write(f'call "{vcvarsall}" x64 >nul\r\n')
            handle.write("if errorlevel 1 exit /b %errorlevel%\r\n")
            handle.write(command + "\r\n")
            temp_path = handle.name
        try:
            return subprocess.run(
                ["cmd.exe", "/d", "/c", temp_path],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
        finally:
            try:
                pathlib.Path(temp_path).unlink(missing_ok=True)
            except OSError:
                pass
    return subprocess.run(
        ["cmd.exe", "/d", "/c", command],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def read_env_var_from_cmd(var_name: str, vcvarsall: str | None = None) -> str | None:
    proc = run_cmd_chain(f"set {var_name}", vcvarsall=vcvarsall)
    if proc.returncode != 0:
        return None
    for line in cleaned_lines(proc.stdout):
        prefix = f"{var_name}="
        if line.startswith(prefix):
            return line[len(prefix) :]
    return None


def collect_msvc_shell_evidence(vcvarsall: str | None = None) -> dict[str, object]:
    details: dict[str, object] = {
        "mode": "vcvarsall_x64" if vcvarsall else "current_shell",
    }
    if vcvarsall:
        details["vcvarsall_bat"] = vcvarsall

    for env_key in TOOLCHAIN_ENV_KEYS:
        value = (
            read_env_var_from_cmd(env_key, vcvarsall)
            if vcvarsall
            else normalize_string(os.environ.get(env_key))
        )
        if value:
            details[env_key] = value

    where_proc = run_cmd_chain("where cl", vcvarsall=vcvarsall)
    where_lines = cleaned_lines(where_proc.stdout)
    if where_lines:
        details["where_cl"] = where_lines
        details["compiler_path"] = where_lines[0]
        install_root = infer_vs_installation_root(where_lines[0])
        if install_root:
            details["compiler_installation_root"] = install_root
    if where_proc.returncode != 0:
        details["where_cl_error"] = completed_output(where_proc) or str(where_proc.returncode)

    cl_bv_proc = run_cmd_chain("cl /Bv", vcvarsall=vcvarsall)
    cl_bv_output = completed_output(cl_bv_proc)
    if cl_bv_output:
        details["cl_bv"] = cl_bv_output
    details["cl_bv_exit_code"] = cl_bv_proc.returncode
    return details


def load_json_report(path: pathlib.Path) -> dict[str, object] | None:
    if not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def load_json_file(path: pathlib.Path) -> dict[str, object] | None:
    if not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def file_contains_all(path: pathlib.Path, needles: list[str]) -> bool:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return False
    return all(needle in text for needle in needles)


def validate_fallback_telemetry_sample(sample_path: pathlib.Path) -> bool:
    if not sample_path.exists() or not GRX008_FALLBACK_TELEMETRY_SCRIPT.exists():
        return False
    return (
        _bench_script_exit_code(
            GRX008_FALLBACK_TELEMETRY_SCRIPT,
            ["--validate-only", str(sample_path)],
        )
        == 0
    )


def grx009_patch_stack_result() -> dict[str, object]:
    return evaluate_patch_stack(
        ROOT,
        EXTERNAL_GODOT,
        GRX009_PATCH_0001,
        GRX009_PATCH_0002,
        GRX009_PATCH_0003,
    )


def grx009_patch_stack_ready(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_stack_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_compile_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_COMPILE_EVIDENCE)


def sha256_of_file(path: pathlib.Path) -> str | None:
    """真实读取文件内容重算 SHA-256(evidence 中记录值须与之匹配,防造假)。"""
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(65536), b""):
                digest.update(chunk)
    except OSError:
        return None
    return digest.hexdigest()


def read_text_prefix(path: pathlib.Path, limit: int = 65536) -> str:
    if not path.is_file():
        return ""
    try:
        return path.read_bytes()[:limit].decode("utf-8", errors="ignore")
    except OSError:
        return ""


def grx009_dxil_artifact_is_real_container(path: pathlib.Path) -> bool:
    text = read_text_prefix(path)
    if text.startswith("; ModuleID"):
        return False
    if "target triple = \"dxil-unknown-shadermodel" in text:
        return False
    if "entry:\n  ret void" in text or "entry:\r\n  ret void" in text:
        return False
    return path.is_file()


def grx009_compile_stderr_has_skip_marker(evidence: dict[str, object]) -> bool:
    commands = evidence.get("commands")
    if not isinstance(commands, list):
        return False
    for command in commands:
        if not isinstance(command, dict):
            continue
        stderr_path = normalize_string(command.get("stderr_path"))
        if not stderr_path:
            continue
        stderr_text = read_text_prefix(ROOT / stderr_path).lower()
        if any(
            marker in stderr_text
            for marker in ("patched llc not found", "dxc validator not found", "skipped")
        ):
            return True
    return False


def grx009_compile_manifest_consistency_issue(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> str | None:
    evidence_status = normalize_string(evidence.get("status"))
    evidence_blocker = normalize_string(evidence.get("blocker_category"))
    manifest_status = normalize_string(manifest.get("offline_compile_status"))
    if manifest_status != evidence_status:
        return (
            "GRX-009 segment 3a manifest/evidence mismatch: "
            f"manifest offline_compile_status={manifest_status or 'missing'} "
            f"but latest evidence status={evidence_status or 'missing'}"
        )
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return "GRX-009 segment 3a manifest/evidence mismatch: implementation_status is missing"
    last_result = normalize_string(implementation_status.get("segment_3a_last_result"))
    if last_result != evidence_status:
        return (
            "GRX-009 segment 3a manifest/evidence mismatch: "
            f"segment_3a_last_result={last_result or 'missing'} "
            f"but latest evidence status={evidence_status or 'missing'}"
        )
    if evidence_status in {"compile_failed", "toolchain_missing"}:
        blockers = manifest.get("compile_blockers")
        blocker_categories: list[str] = []
        if isinstance(blockers, list):
            for blocker in blockers:
                if isinstance(blocker, dict):
                    category = normalize_string(blocker.get("category"))
                    if category:
                        blocker_categories.append(category)
        if evidence_blocker not in blocker_categories:
            return (
                "GRX-009 segment 3a manifest/evidence mismatch: "
                f"latest evidence blocker={evidence_blocker or 'missing'} "
                f"but manifest compile_blockers={blocker_categories or ['missing']}"
            )
    return None


def grx009_segment3a_compile_ready() -> bool:
    manifest = grx009_manifest()
    evidence = grx009_compile_evidence()
    if manifest is None or evidence is None:
        return False
    if not GRX009_COMPILE_SCHEMA.exists():
        return False
    if evidence.get("pass_id") != "luminance_reduction":
        return False
    if evidence.get("status") != "success":
        return False
    if evidence.get("runtime_state") != "fallback_only":
        return False
    if grx009_compile_manifest_consistency_issue(manifest, evidence) is not None:
        return False
    # success evidence 必须已把 manifest 推进到 segment 3(与 schema allOf 同口径)。
    if evidence.get("manifest_segment_after_run") != 3:
        return False
    if manifest.get("offline_compile_status") != "success":
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment") != 3:
        return False
    if implementation_status.get("real_gpu_pass") is not False:
        return False
    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, dict):
        return False
    # manifest 声明的 artifact 路径集合(evidence 路径须与其对应字段一致,防漂移)。
    manifest_artifacts = manifest.get("offline_compile_artifacts")
    if not isinstance(manifest_artifacts, dict):
        return False
    for key in ("dxil", "root_signature", "descriptor_layout"):
        artifact = artifacts.get(key)
        if not isinstance(artifact, dict):
            return False
        path_text = normalize_string(artifact.get("path"))
        if artifact.get("exists") is not True or not path_text:
            return False
        if artifact.get("produced_by_current_run") is not True:
            return False
        # evidence 路径 == manifest.offline_compile_artifacts 对应字段(normalize 后)。
        manifest_path_text = normalize_string(manifest_artifacts.get(key))
        if manifest_path_text != path_text:
            return False
        candidate = ROOT / path_text
        if not candidate.is_file():
            return False
        # evidence 记录的 sha256 须为非空字符串,且与真实文件内容重算值匹配。
        recorded_sha = normalize_string(artifact.get("sha256"))
        if not recorded_sha:
            return False
        actual_sha = sha256_of_file(candidate)
        if actual_sha is None or actual_sha != recorded_sha:
            return False
        if key == "dxil":
            if artifact.get("artifact_kind") != "dxil_container":
                return False
            if artifact.get("semantic_status") == "entry_shell_only":
                return False
            if not grx009_dxil_artifact_is_real_container(candidate):
                return False
    if grx009_compile_stderr_has_skip_marker(evidence):
        return False
    return True


def grx009_manifest() -> dict[str, object] | None:
    return load_json_file(GRX009_PASS_MANIFEST)


def grx009_manifest_implementation_status(
    manifest: dict[str, object],
) -> dict[str, object] | None:
    implementation_status = manifest.get("implementation_status")
    return implementation_status if isinstance(implementation_status, dict) else None


def command_has_required_scons_args(command: object) -> bool:
    candidate = normalize_string(command)
    if not candidate:
        return False
    return all(arg in candidate for arg in REQUIRED_SCONS_ARGS)


def build_summary_primary_command(build_summary: dict[str, object] | None) -> str | None:
    if not isinstance(build_summary, dict):
        return None
    direct_command = normalize_string(build_summary.get("command"))
    if direct_command:
        return direct_command
    attempts = build_summary.get("attempts")
    if not isinstance(attempts, list):
        return None
    successful_command = None
    fallback_command = None
    for attempt in attempts:
        if not isinstance(attempt, dict):
            continue
        command = normalize_string(attempt.get("command"))
        if not command:
            continue
        fallback_command = command
        if attempt.get("exit_code") == 0:
            successful_command = command
            break
    return successful_command or fallback_command


def build_summary_required_scons_args_satisfied(
    build_summary: dict[str, object] | None,
) -> bool:
    if not isinstance(build_summary, dict):
        return False
    explicit_ready = build_summary.get("required_scons_args_satisfied")
    if isinstance(explicit_ready, bool):
        return explicit_ready
    explicit_path_ready = build_summary.get("path_overrides_ready")
    if isinstance(explicit_path_ready, bool):
        return explicit_path_ready
    primary_command = build_summary_primary_command(build_summary)
    ice_workaround_command = normalize_string(build_summary.get("ice_workaround_command"))
    return command_has_required_scons_args(
        primary_command
    ) and command_has_required_scons_args(ice_workaround_command)


def build_summary_has_required_artifacts(build_summary: dict[str, object] | None) -> bool:
    if not isinstance(build_summary, dict):
        return False
    if build_summary.get("artifacts_complete") is not True:
        return False
    artifacts = build_summary.get("artifacts")
    if not isinstance(artifacts, dict):
        return False
    for key in REQUIRED_BUILD_ARTIFACT_KEYS:
        artifact = artifacts.get(key)
        if (
            not isinstance(artifact, dict)
            or artifact.get("exists") is not True
            or normalize_string(artifact.get("path")) is None
            or artifact.get("size_bytes") is None
            or normalize_string(artifact.get("mtime_utc")) is None
            or normalize_string(artifact.get("sha256")) is None
        ):
            return False
    return True


def load_smoke_summary_is_success(load_smoke_summary: dict[str, object] | None) -> bool:
    if not isinstance(load_smoke_summary, dict):
        return False
    return load_smoke_summary.get("status") == "success"


def bench_scenes_ready(bench_smoke_summary: dict[str, object] | None) -> bool:
    if not isinstance(bench_smoke_summary, dict):
        return False
    return (
        bench_smoke_summary.get("status") == "success"
        and bench_smoke_summary.get("scene_count") == 7
        and bench_smoke_summary.get("failure_count") == 0
    )


def bench_runner_ready(bench_runner_summary: dict[str, object] | None) -> bool:
    if not isinstance(bench_runner_summary, dict):
        return False
    return (
        bench_runner_summary.get("status") == "success"
        and bench_runner_summary.get("scene_count") == 7
        and bench_runner_summary.get("failure_count") == 0
    )


def _perf_gate_exit_code(args: list[str]) -> int | None:
    """Run perf_gate.py with args (cwd=ROOT). Returns exit code, or None on error."""
    try:
        proc = subprocess.run(
            [sys.executable, str(GRX006_PERF_GATE_SCRIPT), *args],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
    except OSError:
        return None
    return proc.returncode


def grx006_schema_ready() -> bool:
    """GRX-006 schema/sample validation is available and the gate scripts work.

    Evidence-based: every tracked GRX-006 schema and base sample file must exist
    and parse as JSON, AND the GRX-006 red/green sample commands must behave as
    expected (green sample validates, red samples fail). This is not
    "file exists = done"; unparseable files or a broken gate script keep the
    readiness false so the probe does not advance to GRX-007 prematurely.
    """
    for path in GRX006_SCHEMA_SAMPLE_FILES:
        if not path.exists():
            return False
        try:
            json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return False

    if not GRX006_PERF_GATE_SCRIPT.exists():
        return False
    for sample in (
        GRX006_BASELINE_SMOKE_SAMPLE,
        GRX006_FORBIDDEN_SKIP_SAMPLE,
        GRX006_MISSING_SAMPLE_COUNT_SAMPLE,
    ):
        if not sample.exists():
            return False

    # Green: baseline smoke sample validates (exit 0).
    green = _perf_gate_exit_code(
        ["--kind", "baseline", "--validate-only", str(GRX006_BASELINE_SMOKE_SAMPLE)]
    )
    if green != 0:
        return False
    # Red: forbidden SKIP marker under --strict must fail (non-zero).
    strict_red = _perf_gate_exit_code(["--strict", str(GRX006_FORBIDDEN_SKIP_SAMPLE)])
    if strict_red is None or strict_red == 0:
        return False
    # Red: missing sample_count under baseline validation must fail (non-zero).
    baseline_red = _perf_gate_exit_code(
        [
            "--kind",
            "baseline",
            "--validate-only",
            str(GRX006_MISSING_SAMPLE_COUNT_SAMPLE),
        ]
    )
    if baseline_red is None or baseline_red == 0:
        return False
    return True


def _bench_script_exit_code(script: pathlib.Path, args: list[str]) -> int | None:
    """Run a bench script with args (cwd=ROOT). Returns exit code, or None on error."""
    try:
        proc = subprocess.run(
            [sys.executable, str(script), *args],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
    except OSError:
        return None
    return proc.returncode


def grx007_visual_ready() -> bool:
    """GRX-007 visual diff scaffold/hardening is available and red/green behave.

    Evidence-based: the visual_diff.py script, schema, and tracked samples must
    exist and parse as JSON, AND the visual diff red/green sample commands must
    behave as expected (placeholder + matching LDR pass validate; missing ldr,
    mismatch, and skip-with-fake-ldr fail). This is not "file exists = done".
    """
    for path in (
        GRX007_VISUAL_DIFF_SCRIPT,
        GRX007_VISUAL_SCHEMA,
        GRX007_VISUAL_PLACEHOLDER_SAMPLE,
        GRX007_VISUAL_LDR_PASS_SAMPLE,
        GRX007_VISUAL_MISSING_LDR_SAMPLE,
        GRX007_VISUAL_MISMATCH_SAMPLE,
        GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE,
        GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE,
    ):
        if not path.exists():
            return False
    for json_path in (
        GRX007_VISUAL_SCHEMA,
        GRX007_VISUAL_PLACEHOLDER_SAMPLE,
        GRX007_VISUAL_LDR_PASS_SAMPLE,
        GRX007_VISUAL_MISSING_LDR_SAMPLE,
        GRX007_VISUAL_MISMATCH_SAMPLE,
        GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE,
        GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE,
    ):
        try:
            json.loads(json_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return False

    # Green: placeholder validates (exit 0).
    if _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        ["--validate-only", str(GRX007_VISUAL_PLACEHOLDER_SAMPLE)],
    ) != 0:
        return False
    # Green: recorded ldr_diff matches the computed diff (exit 0).
    if _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT, [str(GRX007_VISUAL_LDR_PASS_SAMPLE)]
    ) != 0:
        return False
    # Red: status=pass missing ldr_diff must FORMAT FAIL (non-zero).
    missing_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        ["--validate-only", str(GRX007_VISUAL_MISSING_LDR_SAMPLE)],
    )
    if missing_red is None or missing_red == 0:
        return False
    # Red: recorded ldr_diff mismatched computed diff must DIFF FAIL (non-zero).
    mismatch_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT, [str(GRX007_VISUAL_MISMATCH_SAMPLE)]
    )
    if mismatch_red is None or mismatch_red == 0:
        return False
    # Red: skip frame carrying a fabricated ldr_diff must FORMAT FAIL (non-zero).
    skip_fake_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        ["--validate-only", str(GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE)],
    )
    if skip_fake_red is None or skip_fake_red == 0:
        return False
    # Red: status=pass frame whose reference/candidate artifacts are missing on
    # disk must DIFF FAIL (non-zero); it must not be downgraded to SKIP.
    missing_frame_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        [str(GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE)],
    )
    if missing_frame_red is None or missing_frame_red == 0:
        return False
    return True


def grx008_telemetry_ready() -> bool:
    """GRX-008 fallback telemetry scaffold/hardening is available and red/green behave.

    Evidence-based: the fallback_telemetry.py script, schema, and tracked samples
    must exist and parse as JSON, AND the red/green sample commands must behave as
    expected (scaffold placeholder validates; full/measured_local with null
    timestamp and scaffold with inactive fallback both fail). This is not
    "file exists = done".
    """
    for path in (
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        GRX008_FALLBACK_SCHEMA,
        GRX008_FALLBACK_PLACEHOLDER_SAMPLE,
        GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE,
        GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE,
    ):
        if not path.exists():
            return False
    for json_path in (
        GRX008_FALLBACK_SCHEMA,
        GRX008_FALLBACK_PLACEHOLDER_SAMPLE,
        GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE,
        GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE,
    ):
        try:
            json.loads(json_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return False

    # Green: scaffold placeholder validates (exit 0).
    if _bench_script_exit_code(
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        ["--validate-only", str(GRX008_FALLBACK_PLACEHOLDER_SAMPLE)],
    ) != 0:
        return False
    # Red: full/measured_local with null timestamp/frame must FORMAT FAIL.
    full_null_red = _bench_script_exit_code(
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        ["--validate-only", str(GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE)],
    )
    if full_null_red is None or full_null_red == 0:
        return False
    # Red: scaffold with godot_fallback_active=false must FORMAT FAIL.
    scaffold_inactive_red = _bench_script_exit_code(
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        ["--validate-only", str(GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE)],
    )
    if scaffold_inactive_red is None or scaffold_inactive_red == 0:
        return False
    return True


def grx009_manifest_godot_files(manifest: dict[str, object]) -> list[str] | None:
    """Collect the Godot-relative files recorded by the GRX-009 manifest.

    Returns the header/source/shader/call-site file paths recorded under
    godot_hook_investigation, or None when any of them is missing, empty, or
    not a string. The paths are investigation records only; nothing here
    mutates external/godot-master.
    """
    investigation = manifest.get("godot_hook_investigation")
    if not isinstance(investigation, dict):
        return None
    files: list[str] = []
    effect_class = investigation.get("effect_class")
    if not isinstance(effect_class, dict):
        return None
    for key in ("header", "source"):
        value = normalize_string(effect_class.get(key))
        if not value:
            return None
        files.append(value)
    shaders = investigation.get("shaders")
    if not isinstance(shaders, list) or not shaders:
        return None
    for shader in shaders:
        value = normalize_string(shader)
        if not value:
            return None
        files.append(value)
    call_sites = investigation.get("call_sites")
    if not isinstance(call_sites, list) or not call_sites:
        return None
    for call_site in call_sites:
        if not isinstance(call_site, dict):
            return None
        value = normalize_string(call_site.get("file"))
        if not value:
            return None
        files.append(value)
    return files


def grx009_manifest_godot_files_exist(manifest: dict[str, object]) -> bool:
    """Every manifest-recorded Godot file must exist under external/godot-master.

    Read-only check: each recorded header/source/shader/call-site path must be
    a relative path that resolves to an existing file inside the ignored Godot
    snapshot. Absolute paths or paths escaping the snapshot root fail.
    """
    files = grx009_manifest_godot_files(manifest)
    if files is None:
        return False
    try:
        external_root = EXTERNAL_GODOT.resolve()
    except OSError:
        return False
    for rel in files:
        if pathlib.PurePath(rel).is_absolute():
            return False
        candidate = EXTERNAL_GODOT / rel
        try:
            resolved = candidate.resolve()
        except OSError:
            return False
        if not resolved.is_relative_to(external_root):
            return False
        if not candidate.is_file():
            return False
    return True


def grx009_prep_ready() -> bool:
    """GRX-009 luminance reduction pass preparation artifacts are present.

    Evidence-based: the PASS_CONTRACT.md and pass_manifest.json preparation
    artifacts must exist, the manifest must parse as JSON, it must declare the
    default-disabled luminance_reduction pass (pass_id, implemented=false,
    default disabled, target scenes post_fx_chain + mixed_forward_plus), and
    every Godot source/header/shader/call-site file recorded in
    godot_hook_investigation must exist under external/godot-master (checked
    read-only; the snapshot is never modified). Readiness here means the
    preparation record is real; it does NOT mean any acceleration pass is
    implemented.
    """
    if not GRX009_PASS_CONTRACT.exists() or not GRX009_PASS_MANIFEST.exists():
        return False
    manifest = grx009_manifest()
    if manifest is None:
        return False
    if manifest.get("pass_id") != "luminance_reduction":
        return False
    if manifest.get("implemented") is not False:
        return False
    if manifest.get("default_enable_state") != "disabled":
        return False
    target_scenes = manifest.get("target_scenes")
    if not isinstance(target_scenes, list):
        return False
    if "post_fx_chain" not in target_scenes or "mixed_forward_plus" not in target_scenes:
        return False
    return grx009_manifest_godot_files_exist(manifest)


def grx009_segment1_ready() -> bool:
    """GRX-009 segment 1 gated scaffold artifacts are present and coherent.

    Evidence-based: the prep artifacts must already be valid, the manifest must
    still declare implementation_status.segment == 1 with
    godot_core_call_site_wired == false and real_gpu_pass == false, the 0002
    module patch must carry the expected luminance gate markers, the disabled
    telemetry sample must exist and parse, and the Rust bridge must still carry
    the LuminanceReductionGate marker. This remains a historical gate once the
    manifest advances to segment 2.
    """
    if not grx009_prep_ready():
        return False
    manifest = grx009_manifest()
    if manifest is None:
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment") != 1:
        return False
    if implementation_status.get("real_gpu_pass") is not False:
        return False
    if implementation_status.get("godot_core_call_site_wired") is not False:
        return False
    if not file_contains_all(
        GRX009_PATCH_0002,
        [
            "rendering/rurix_accel/passes/luminance_reduction/enabled",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "rxgd_record_pass",
            "try_record_luminance_reduction",
        ],
    ):
        return False
    if not validate_fallback_telemetry_sample(GRX009_DISABLED_TELEMETRY_SAMPLE):
        return False
    if not file_contains_all(GRX009_BRIDGE_LIB, ["LuminanceReductionGate"]):
        return False
    return True


def grx009_segment2_ready(patch_stack_result: dict[str, object] | None = None) -> bool:
    """GRX-009 segment 2 core call-site fallback wiring is present and coherent.

    Evidence-based: the prep artifacts must already be valid, the manifest must
    declare implementation_status.segment == 2 with
    godot_core_call_site_wired == true and real_gpu_pass == false, the segment 1
    bridge/module markers must still exist, the new 0003 core call-site patch
    must carry the expected D3D12Hooks + renderer_scene_render_rd wiring
    markers, and the segment-2 scaffold telemetry sample must exist and parse.
    """
    if not grx009_prep_ready():
        return False
    manifest = grx009_manifest()
    if manifest is None:
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment") != 2:
        return False
    if implementation_status.get("real_gpu_pass") is not False:
        return False
    if implementation_status.get("godot_core_call_site_wired") is not True:
        return False
    if not file_contains_all(
        GRX009_PATCH_0002,
        [
            "rendering/rurix_accel/passes/luminance_reduction/enabled",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "rxgd_record_pass",
            "try_record_luminance_reduction",
        ],
    ):
        return False
    if not file_contains_all(GRX009_BRIDGE_LIB, ["LuminanceReductionGate"]):
        return False
    if not grx009_patch_stack_ready(patch_stack_result):
        return False
    if not file_contains_all(
        GRX009_PATCH_0003,
        [
            "drivers/d3d12/d3d12_hooks.h",
            "renderer_scene_render_rd.cpp",
            "D3D12Hooks::get_singleton",
            "try_record_luminance_reduction",
            "override",
        ],
    ):
        return False
    if not validate_fallback_telemetry_sample(GRX009_CALLSITE_WIRED_TELEMETRY_SAMPLE):
        return False
    return True


def run_probe(name: str, cmd: list[str], *, ok_status: str = "PASS") -> ProbeResult:
    try:
        proc = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, check=False)
    except FileNotFoundError as exc:
        return ProbeResult(name, "SKIP", f"command not found: {exc.filename or cmd[0]}", {})

    output = completed_output(proc)
    if proc.returncode == 0:
        reason = output or "command succeeded"
        return ProbeResult(name, ok_status, reason, {"command": " ".join(cmd)})
    return ProbeResult(
        name,
        "SKIP",
        output or f"command failed with exit code {proc.returncode}",
        {"command": " ".join(cmd), "exit_code": proc.returncode},
    )


def find_paths_from_env(paths: str | None) -> list[pathlib.Path]:
    if not paths:
        return []
    found: list[pathlib.Path] = []
    for item in paths.split(os.pathsep):
        item = item.strip().strip('"')
        if item:
            found.append(pathlib.Path(item))
    return found


def newest_subdir(root: pathlib.Path) -> pathlib.Path | None:
    if not root.exists():
        return None
    subdirs = sorted(path for path in root.iterdir() if path.is_dir())
    return subdirs[-1] if subdirs else None


def find_windows_sdk_versions(root: pathlib.Path) -> list[pathlib.Path]:
    if not root.exists():
        return []
    return sorted((path for path in root.iterdir() if path.is_dir()), reverse=True)


def probe_godot_tree() -> list[ProbeResult]:
    results: list[ProbeResult] = []
    if EXTERNAL_GODOT.exists():
        results.append(
            ProbeResult(
                "godot_snapshot",
                "PASS",
                "external/godot-master exists",
                {"path": str(EXTERNAL_GODOT)},
            )
        )
    else:
        results.append(
            ProbeResult(
                "godot_snapshot",
                "FAIL",
                "external/godot-master is missing",
                {"path": str(EXTERNAL_GODOT)},
            )
        )

    if SCONSTRUCT.exists():
        results.append(
            ProbeResult(
                "godot_sconstruct",
                "PASS",
                "SConstruct exists",
                {"path": str(SCONSTRUCT)},
            )
        )
    else:
        results.append(
            ProbeResult(
                "godot_sconstruct",
                "FAIL",
                "SConstruct is missing",
                {"path": str(SCONSTRUCT)},
            )
        )
    return results


def probe_vs_build_tools() -> ProbeResult:
    details: dict[str, object] = {}
    env_install = os.environ.get("VSINSTALLDIR")
    if env_install:
        details["VSINSTALLDIR"] = env_install

    if VSWHERE.exists():
        details["vswhere"] = str(VSWHERE)
        proc = subprocess.run(
            [
                str(VSWHERE),
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-format",
                "json",
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        output = completed_output(proc)
        if proc.returncode != 0:
            return ProbeResult(
                "vs_build_tools",
                "SKIP",
                output or f"vswhere failed with exit code {proc.returncode}",
                details,
            )
        try:
            installs = json.loads(proc.stdout or "[]")
        except json.JSONDecodeError:
            installs = []
        if not installs:
            return ProbeResult(
                "vs_build_tools",
                "SKIP",
                "vswhere did not find a Visual Studio installation with VC tools",
                details,
            )

        install_records: list[dict[str, object]] = []
        for install_record in installs[:8]:
            if not isinstance(install_record, dict):
                continue
            record: dict[str, object] = {}
            for key in (
                "displayName",
                "installationName",
                "installationPath",
                "installationVersion",
                "productId",
                "isPrerelease",
            ):
                value = install_record.get(key)
                if value not in (None, ""):
                    record[key] = value
            if record:
                install_records.append(record)
        if install_records:
            details["installations"] = install_records

        selected_install = next(
            (
                install_record
                for install_record in installs
                if isinstance(install_record, dict)
                and normalize_string(install_record.get("installationPath"))
            ),
            None,
        )
        if not isinstance(selected_install, dict):
            return ProbeResult(
                "vs_build_tools",
                "SKIP",
                "vswhere did not return a usable installationPath",
                details,
            )

        install_path = pathlib.Path(str(selected_install["installationPath"]))
        details["selected_installation_path"] = str(install_path)
        details["installation_path"] = str(install_path)
        for key in ("displayName", "installationName", "installationVersion", "productId"):
            value = selected_install.get(key)
            if value not in (None, ""):
                details[f"selected_{key}"] = value
        vcvarsall = install_path / VCVARSALL_REL
        if vcvarsall.exists():
            details["vcvarsall_bat"] = str(vcvarsall)
        msvc_toolset = newest_subdir(install_path / "VC" / "Tools" / "MSVC")
        if msvc_toolset:
            details["msvc_toolset"] = str(msvc_toolset)
            candidate_cl = msvc_toolset / "bin" / "Hostx64" / "x64" / "cl.exe"
            if candidate_cl.exists():
                details["selected_candidate_cl"] = str(candidate_cl)
                details["candidate_cl"] = str(candidate_cl)
        return ProbeResult(
            "vs_build_tools",
            "PASS",
            "Visual Studio/Build Tools with VC tools were found via vswhere",
            details,
        )

    if env_install:
        install_path = pathlib.Path(env_install)
        vcvarsall = install_path / VCVARSALL_REL
        if vcvarsall.exists():
            details["vcvarsall_bat"] = str(vcvarsall)
        details["selected_installation_path"] = str(install_path)
        details["installation_path"] = str(install_path)
        msvc_toolset = newest_subdir(install_path / "VC" / "Tools" / "MSVC")
        if msvc_toolset:
            details["msvc_toolset"] = str(msvc_toolset)
            candidate_cl = msvc_toolset / "bin" / "Hostx64" / "x64" / "cl.exe"
            if candidate_cl.exists():
                details["selected_candidate_cl"] = str(candidate_cl)
                details["candidate_cl"] = str(candidate_cl)
        return ProbeResult(
            "vs_build_tools",
            "PASS",
            "VSINSTALLDIR is set; assuming this shell comes from a Visual Studio installation",
            details,
        )

    return ProbeResult(
        "vs_build_tools",
        "SKIP",
        "vswhere.exe was not found and VSINSTALLDIR is not set",
        details,
    )


def probe_msvc(vs_probe: ProbeResult) -> ProbeResult:
    details = collect_msvc_shell_evidence()
    for key in ("vswhere", "installation_path", "vcvarsall_bat", "msvc_toolset", "candidate_cl"):
        value = vs_probe.details.get(key)
        if value:
            details[key] = value
    compiler_path = normalize_string(details.get("compiler_path"))
    if compiler_path:
        details["path"] = compiler_path
        output = normalize_string(details.get("cl_bv"))
        return ProbeResult("msvc_cl", "PASS", output or "cl is available", details)

    if details.get("candidate_cl"):
        return ProbeResult(
            "msvc_cl",
            "SKIP",
            "cl is not available on PATH; launch a Developer PowerShell or call vcvarsall.bat first",
            details,
        )

    return ProbeResult(
        "msvc_cl",
        "SKIP",
        "cl is not available on PATH and no usable VC toolset was discovered",
        details,
    )


def probe_msvc_via_vcvarsall(vs_probe: ProbeResult) -> ProbeResult:
    vcvarsall = vs_probe.details.get("vcvarsall_bat")
    if isinstance(vcvarsall, str) and vcvarsall:
        details = collect_msvc_shell_evidence(vcvarsall)
        command = f'cmd.exe /d /s /c "call "{vcvarsall}" x64 >nul && where cl && cl /Bv"'
        details["command"] = command
        compiler_path = normalize_string(details.get("compiler_path"))
        if compiler_path:
            details["path"] = compiler_path
            return ProbeResult(
                "msvc_cl_via_vcvarsall",
                "PASS",
                "cl is available when wrapped with vcvarsall.bat",
                details | {"activation_output": details.get("cl_bv", "cl invocation succeeded")},
            )
        return ProbeResult(
            "msvc_cl_via_vcvarsall",
            "SKIP",
            normalize_string(details.get("where_cl_error"))
            or normalize_string(details.get("cl_bv"))
            or "vcvarsall activation did not expose cl.exe",
            details,
        )

    return ProbeResult(
        "msvc_cl_via_vcvarsall",
        "SKIP",
        "vcvarsall.bat was not discovered",
        details,
    )


def probe_headers() -> ProbeResult:
    include_roots = find_paths_from_env(os.environ.get("INCLUDE"))
    sdk_dir = os.environ.get("WindowsSdkDir")
    sdk_version = os.environ.get("WindowsSdkVersion", "").strip("\\/")
    details: dict[str, object] = {}

    if sdk_dir:
        details["WindowsSdkDir"] = sdk_dir
    if sdk_version:
        details["WindowsSdkVersion"] = sdk_version

    header_hits: dict[str, str] = {}
    search_roots: list[pathlib.Path] = []
    search_roots.extend(include_roots)

    if sdk_dir and sdk_version:
        search_roots.append(pathlib.Path(sdk_dir) / "Include" / sdk_version / "um")
        search_roots.append(pathlib.Path(sdk_dir) / "Include" / sdk_version / "shared")

    for root in DEFAULT_SDK_INCLUDE_ROOTS:
        for version_dir in find_windows_sdk_versions(root):
            search_roots.append(version_dir / "um")
            search_roots.append(version_dir / "shared")

    deduped_roots: list[pathlib.Path] = []
    seen: set[str] = set()
    for root in search_roots:
        key = str(root)
        if key not in seen:
            seen.add(key)
            deduped_roots.append(root)

    for header in HEADER_CANDIDATES:
        for root in deduped_roots:
            candidate = root / header
            if candidate.exists():
                header_hits[header] = str(candidate)
                break

    details.update(header_hits)
    if all(header in header_hits for header in HEADER_CANDIDATES):
        return ProbeResult(
            "windows_sdk_d3d12_headers",
            "PASS",
            "required Windows SDK D3D12 headers were found",
            details,
        )

    missing = [header for header in HEADER_CANDIDATES if header not in header_hits]
    details["searched_roots"] = [str(path) for path in deduped_roots[:12]]
    return ProbeResult(
        "windows_sdk_d3d12_headers",
        "SKIP",
        f"missing headers: {', '.join(missing)}",
        details,
    )


def probe_tool_path(tool_name: str) -> ProbeResult:
    on_path = shutil.which(tool_name)
    details: dict[str, object] = {}
    if on_path:
        return ProbeResult(
            tool_name.lower().removesuffix(".exe"),
            "PASS",
            f"{tool_name} found on PATH",
            {"path": on_path},
        )

    for root in DEFAULT_SDK_BIN_ROOTS:
        if not root.exists():
            continue
        for version_dir in find_windows_sdk_versions(root):
            for arch in ("x64", "x86"):
                candidate = version_dir / arch / tool_name
                if candidate.exists():
                    details["path"] = str(candidate)
                    return ProbeResult(
                        tool_name.lower().removesuffix(".exe"),
                        "PASS",
                        f"{tool_name} found in Windows SDK bin",
                        details,
                    )

    return ProbeResult(
        tool_name.lower().removesuffix(".exe"),
        "SKIP",
        f"{tool_name} was not found on PATH or common Windows SDK bin paths",
        details,
    )


def probe_rurix_godot_dll() -> ProbeResult:
    if RURIX_GODOT_DLL.exists():
        return ProbeResult(
            "rurix_godot_dll",
            "PASS",
            "target/debug/rurix_godot.dll exists",
            {"path": str(RURIX_GODOT_DLL)},
        )
    return ProbeResult(
        "rurix_godot_dll",
        "SKIP",
        "target/debug/rurix_godot.dll is missing; actual buildability is verified by cargo build later",
        {"path": str(RURIX_GODOT_DLL)},
    )


def render_command(parts: list[str]) -> str:
    return subprocess.list2cmdline(parts)


def render_godot_local_command(parts: list[str]) -> str:
    base = render_command(parts)
    return f"set LOCALAPPDATA={LOCAL_GODOT_LOCALAPPDATA} && {base}"


def wrap_with_localappdata(cmd: str) -> str:
    return f"$env:LOCALAPPDATA='{LOCAL_GODOT_LOCALAPPDATA}'; {cmd}"


def first_match(root: pathlib.Path, pattern: str) -> pathlib.Path | None:
    if not root.exists():
        return None
    return next(root.glob(pattern), None)


def first_recursive_match(root: pathlib.Path, pattern: str) -> pathlib.Path | None:
    if not root.exists():
        return None
    return next(root.rglob(pattern), None)


def probe_godot_accesskit_deps() -> ProbeResult:
    accesskit_root = LOCAL_GODOT_BUILD_DEPS / "accesskit"
    include_dir = accesskit_root / "include"
    lib_dir = (
        accesskit_root
        / "lib"
        / "windows"
        / ACCESSKIT_WINDOWS_ARCH
        / "msvc"
        / "static"
    )
    accesskit_lib = first_match(lib_dir, "accesskit*.lib")
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "accesskit_sdk_path": str(accesskit_root),
        "include_dir": str(include_dir),
        "lib_dir": str(lib_dir),
        "arch": GODOT_WINDOWS_ARCH,
        "recommended_install_command": render_godot_local_command(
            ["py", "-3", str(GODOT_INSTALL_ACCESSKIT)]
        ),
    }
    if include_dir.exists() and accesskit_lib is not None:
        details["library"] = str(accesskit_lib)
        return ProbeResult(
            "godot_accesskit_deps",
            "PASS",
            "workspace-local AccessKit SDK was found",
            details,
        )
    return ProbeResult(
        "godot_accesskit_deps",
        "SKIP",
        "workspace-local AccessKit SDK is missing",
        details,
    )


def probe_godot_d3d12_deps() -> ProbeResult:
    mesa_arch_root = LOCAL_GODOT_BUILD_DEPS / f"mesa-{GODOT_WINDOWS_ARCH}-msvc"
    mesa_fallback_root = LOCAL_GODOT_BUILD_DEPS / "mesa"
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "mesa_arch_path": str(mesa_arch_root),
        "mesa_fallback_path": str(mesa_fallback_root),
        "arch": GODOT_WINDOWS_ARCH,
        "recommended_install_command": render_godot_local_command(
            ["py", "-3", str(GODOT_INSTALL_D3D12_DEPS)]
        ),
    }

    candidates = [
        mesa_arch_root / "bin",
        mesa_fallback_root / "bin",
    ]
    for bin_dir in candidates:
        if not bin_dir.exists():
            continue
        libnir = first_match(bin_dir, f"libNIR.windows.{GODOT_WINDOWS_ARCH}*")
        if libnir is None:
            libnir = first_match(bin_dir, "libNIR.windows.*")
        if libnir is not None:
            details["bin_dir"] = str(bin_dir)
            details["libnir"] = str(libnir)
            return ProbeResult(
                "godot_d3d12_deps",
                "PASS",
                "workspace-local Mesa/NIR D3D12 build deps were found",
                details,
            )

    return ProbeResult(
        "godot_d3d12_deps",
        "SKIP",
        "workspace-local Mesa/NIR D3D12 build deps are missing",
        details,
    )


def probe_godot_agility_sdk() -> ProbeResult:
    agility_root = LOCAL_GODOT_BUILD_DEPS / "agility_sdk"
    expected_dir = agility_root / "build" / "native" / "bin" / AGILITY_WINDOWS_ARCH
    d3d12core = expected_dir / "D3D12Core.dll"
    sdk_layers = expected_dir / "d3d12SDKLayers.dll"
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "agility_sdk_path": str(agility_root),
        "arch": AGILITY_WINDOWS_ARCH,
        "expected_dir": str(expected_dir),
    }
    mismatched_hits: dict[str, dict[str, str]] = {}
    for arch_name in ("x64", "arm64", "win32"):
        if arch_name == AGILITY_WINDOWS_ARCH:
            continue
        arch_dir = agility_root / "build" / "native" / "bin" / arch_name
        arch_hits: dict[str, str] = {}
        core_candidate = arch_dir / "D3D12Core.dll"
        layers_candidate = arch_dir / "d3d12SDKLayers.dll"
        if core_candidate.exists():
            arch_hits["D3D12Core.dll"] = str(core_candidate)
        if layers_candidate.exists():
            arch_hits["D3D12SDKLayers.dll"] = str(layers_candidate)
        if arch_hits:
            mismatched_hits[str(arch_dir)] = arch_hits
    if mismatched_hits:
        details["mismatched_arch_hits"] = mismatched_hits
    if d3d12core.exists():
        details["D3D12Core.dll"] = str(d3d12core)
    if sdk_layers.exists():
        details["D3D12SDKLayers.dll"] = str(sdk_layers)
    if d3d12core.exists():
        return ProbeResult(
            "godot_agility_sdk",
            "PASS",
            "workspace-local Agility SDK was found for the current architecture",
            details,
        )
    return ProbeResult(
        "godot_agility_sdk",
        "SKIP",
        "workspace-local Agility SDK was not found for the current architecture",
        details,
    )


def probe_godot_pix_runtime() -> ProbeResult:
    pix_root = LOCAL_GODOT_BUILD_DEPS / "pix"
    expected_dir = pix_root / "bin" / PIX_WINDOWS_ARCH
    pix_runtime = expected_dir / "WinPixEventRuntime.dll"
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "pix_path": str(pix_root),
        "arch": PIX_WINDOWS_ARCH,
        "expected_dir": str(expected_dir),
    }
    mismatched_hits: dict[str, dict[str, str]] = {}
    for arch_name in ("x64", "ARM64"):
        if arch_name == PIX_WINDOWS_ARCH:
            continue
        arch_dir = pix_root / "bin" / arch_name
        runtime_candidate = arch_dir / "WinPixEventRuntime.dll"
        if runtime_candidate.exists():
            mismatched_hits[str(arch_dir)] = {"WinPixEventRuntime.dll": str(runtime_candidate)}
    if mismatched_hits:
        details["mismatched_arch_hits"] = mismatched_hits
    if pix_runtime.exists():
        details["WinPixEventRuntime.dll"] = str(pix_runtime)
        return ProbeResult(
            "godot_pix_runtime",
            "PASS",
            "workspace-local PIX runtime was found for the current architecture",
            details,
        )
    return ProbeResult(
        "godot_pix_runtime",
        "SKIP",
        "workspace-local PIX runtime was not found for the current architecture",
        details,
    )


def preferred_scons_info(by_name: dict[str, dict[str, object]]) -> tuple[str | None, str]:
    if by_name.get("scons_cli", {}).get("status") == "PASS":
        return "scons", "existing"
    if by_name.get("python_scons", {}).get("status") == "PASS":
        return render_command(["py", "-3", "-m", "SCons"]), "existing"
    if by_name.get("local_python_scons", {}).get("status") == "PASS":
        return render_command([str(LOCAL_SCONS_PYTHON), "-m", "SCons"]), "workspace-local venv"
    return None, "unavailable"


def shell_wrap_with_vcvars(cmd: str, by_name: dict[str, dict[str, object]]) -> str:
    vcvarsall = by_name.get("vs_build_tools", {}).get("details", {}).get("vcvarsall_bat")
    if not isinstance(vcvarsall, str) or not vcvarsall:
        return cmd
    if by_name.get("msvc_cl", {}).get("status") == "PASS":
        return cmd
    return f'& $env:ComSpec /c \'call "{vcvarsall}" x64 && {cmd}\''


def summarize(results: list[ProbeResult]) -> dict[str, object]:
    by_name = {result.name: asdict(result) for result in results}
    build_summary = load_json_report(BUILD_SUMMARY_REPORT)
    load_smoke_summary = load_json_report(LOAD_SMOKE_SUMMARY_REPORT)
    bench_smoke_summary = load_json_report(BENCH_SMOKE_SUMMARY_REPORT)
    bench_runner_summary = load_json_report(BENCH_RUNNER_SUMMARY_REPORT)
    build_summary_required_args_satisfied = build_summary_required_scons_args_satisfied(
        build_summary
    )
    build_summary_status = (
        normalize_string(build_summary.get("status")) if isinstance(build_summary, dict) else None
    )
    build_summary_primary_cmd = build_summary_primary_command(build_summary)
    build_summary_ice_cmd = normalize_string(
        build_summary.get("ice_workaround_command") if isinstance(build_summary, dict) else None
    )
    build_artifacts_ready = build_summary_has_required_artifacts(
        build_summary
    ) and build_summary_required_args_satisfied
    load_smoke_ready = load_smoke_summary_is_success(load_smoke_summary)
    scenes_ready = bench_scenes_ready(bench_smoke_summary)
    runner_ready = bench_runner_ready(bench_runner_summary)
    grx006_ready = grx006_schema_ready()
    grx007_ready = grx007_visual_ready()
    grx008_ready = grx008_telemetry_ready()
    grx009_ready = grx009_prep_ready()
    grx009_segment1 = grx009_segment1_ready()
    grx009_patch_stack = grx009_patch_stack_result()
    grx009_patch_stack_state = normalize_string(grx009_patch_stack.get("state"))
    grx009_patch_stack_reason = normalize_string(grx009_patch_stack.get("reason"))
    grx009_patch_stack_is_ready = grx009_patch_stack_ready(grx009_patch_stack)
    grx009_segment2 = grx009_segment2_ready(grx009_patch_stack)
    grx009_compile = grx009_compile_evidence()
    grx009_compile_status = (
        normalize_string(grx009_compile.get("status"))
        if isinstance(grx009_compile, dict)
        else None
    )
    grx009_compile_blocker_category = (
        normalize_string(grx009_compile.get("blocker_category"))
        if isinstance(grx009_compile, dict)
        else None
    )
    grx009_compile_blocker_summary = (
        normalize_string(grx009_compile.get("blocker_summary"))
        if isinstance(grx009_compile, dict)
        else None
    )
    grx009_manifest_data = grx009_manifest()
    grx009_compile_manifest_consistency_warning = (
        grx009_compile_manifest_consistency_issue(grx009_manifest_data, grx009_compile)
        if isinstance(grx009_manifest_data, dict) and isinstance(grx009_compile, dict)
        else None
    )
    grx009_segment3a = grx009_segment3a_compile_ready()
    launcher, scons_source = preferred_scons_info(by_name)
    msvc_ready = (
        by_name["msvc_cl"]["status"] == "PASS"
        or by_name.get("msvc_cl_via_vcvarsall", {}).get("status") == "PASS"
    )
    accesskit_ready = by_name["godot_accesskit_deps"]["status"] == "PASS"
    d3d12_deps_ready = by_name["godot_d3d12_deps"]["status"] == "PASS"
    build_ready = (
        launcher is not None
        and by_name["godot_snapshot"]["status"] == "PASS"
        and by_name["godot_sconstruct"]["status"] == "PASS"
        and msvc_ready
        and by_name["windows_sdk_d3d12_headers"]["status"] == "PASS"
        and by_name["dxc"]["status"] == "PASS"
        and accesskit_ready
        and d3d12_deps_ready
        and by_name["rurix_godot_dll"]["status"] == "PASS"
    )

    blockers: list[str] = []
    warnings: list[str] = []
    optional_tools_missing: list[str] = []
    if by_name["godot_snapshot"]["status"] != "PASS":
        blockers.append("missing external/godot-master snapshot")
    if by_name["godot_sconstruct"]["status"] != "PASS":
        blockers.append("missing external/godot-master/SConstruct")
    if launcher is None:
        blockers.append(
            "missing SCons launcher (`scons`, `py -3 -m SCons`, and workspace-local "
            "`target/grx/scons-venv/Scripts/python.exe -m SCons` are all unavailable)"
        )
    if not msvc_ready:
        if by_name.get("msvc_cl_via_vcvarsall", {}).get("status") == "SKIP":
            blockers.append(
                "MSVC toolset was discovered, but `cl` could not be activated even with vcvarsall.bat"
            )
        elif by_name["vs_build_tools"]["status"] == "PASS":
            blockers.append("MSVC toolset was discovered, but `cl` is not active in the current shell")
        else:
            blockers.append("MSVC `cl` was not discovered")
    elif by_name["msvc_cl"]["status"] != "PASS":
        warnings.append(
            "`cl` is not active in the current shell; wrap commands with vcvarsall.bat"
        )
    if by_name["windows_sdk_d3d12_headers"]["status"] != "PASS":
        blockers.append("required Windows SDK D3D12 headers are missing")
    if by_name["dxc"]["status"] != "PASS":
        blockers.append("`dxc.exe` is missing")
    if not accesskit_ready:
        blockers.append(
            "workspace-local AccessKit SDK is missing; install it under "
            "`target/grx/localappdata/Godot/build_deps/accesskit` before running the default Godot SCons build"
        )
    if not d3d12_deps_ready:
        blockers.append(
            "workspace-local Godot Mesa/NIR D3D12 deps are missing; `d3d12=yes` cannot proceed without them"
        )
    if by_name["godot_agility_sdk"]["status"] != "PASS":
        warnings.append(
            "workspace-local Agility SDK was not found; Godot can still configure/build, but runtime packaging may need it later"
        )
    if by_name["godot_pix_runtime"]["status"] != "PASS":
        warnings.append(
            "workspace-local PIX runtime was not found; this is optional unless `use_pix=yes` is requested"
        )
    if by_name["dxv"]["status"] != "PASS":
        optional_tools_missing.append("`dxv.exe` is missing")
        warnings.append(
            "`dxv.exe` is unavailable; this is a later DXIL/device validation warning, not a Godot SCons build blocker"
        )
    if by_name["rurix_godot_dll"]["status"] != "PASS":
        blockers.append("`target/debug/rurix_godot.dll` is missing")
    if build_artifacts_ready and build_summary_status and build_summary_status != "success":
        warnings.append(
            "Latest Godot wrapper build exited nonzero, but required GRX artifacts with "
            "`disable_path_overrides=no` evidence are present; see godot_scons_build_summary.json for failure_targets"
        )
    if grx009_compile_manifest_consistency_warning:
        warnings.append(grx009_compile_manifest_consistency_warning)

    recommended_probe = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            render_command(["py", "-3", str(ROOT / "ci" / "godot_rurix_toolchain_probe.py")]),
            by_name,
        )
    )
    recommended_toolchain_cl = normalize_string(
        by_name.get("vs_build_tools", {}).get("details", {}).get("selected_candidate_cl")
    ) or normalize_string(by_name.get("vs_build_tools", {}).get("details", {}).get("candidate_cl"))
    recommended_toolchain_install = normalize_string(
        by_name.get("vs_build_tools", {}).get("details", {}).get("selected_installation_path")
    ) or normalize_string(by_name.get("vs_build_tools", {}).get("details", {}).get("installation_path"))
    raw_scons_command = None
    if build_ready:
        raw_scons_command = f"{launcher} {SCONS_BUILD_ARGS}"
    recommended_scons = (
        wrap_with_localappdata(shell_wrap_with_vcvars(raw_scons_command, by_name))
        if raw_scons_command
        else None
    )
    raw_ice_workaround_command = f"{launcher} {SCONS_ICE_ARGS}" if launcher else None
    ice_workaround_command = (
        wrap_with_localappdata(shell_wrap_with_vcvars(raw_ice_workaround_command, by_name))
        if raw_ice_workaround_command
        else None
    )
    recommended_accesskit_install = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            render_command(["py", "-3", str(GODOT_INSTALL_ACCESSKIT)]),
            by_name,
        )
    )
    recommended_d3d12_install = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            render_command(["py", "-3", str(GODOT_INSTALL_D3D12_DEPS)]),
            by_name,
        )
    )
    recommended_all_build_deps_install = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            f"py -3 {subprocess.list2cmdline([str(GODOT_INSTALL_D3D12_DEPS)])} && "
            f"py -3 {subprocess.list2cmdline([str(GODOT_INSTALL_ACCESSKIT)])}",
            by_name,
        )
    )
    recommended_dev_shell = None
    vcvarsall = by_name.get("vs_build_tools", {}).get("details", {}).get("vcvarsall_bat")
    if isinstance(vcvarsall, str) and vcvarsall:
        recommended_dev_shell = f'cmd /k ""{vcvarsall}" x64"'

    scons_actual_compiler_path = None
    scons_actual_compiler_source = None
    scons_actual_compiler_install = None
    if isinstance(build_summary, dict):
        scons_actual_compiler_path = normalize_string(build_summary.get("actual_compiler_path"))
        scons_actual_compiler_source = normalize_string(build_summary.get("actual_compiler_source"))
        scons_actual_compiler_install = normalize_string(build_summary.get("actual_compiler_install"))
    if not scons_actual_compiler_path:
        vcvars_details = by_name.get("msvc_cl_via_vcvarsall", {}).get("details", {})
        scons_actual_compiler_path = normalize_string(vcvars_details.get("compiler_path"))
        if scons_actual_compiler_path:
            scons_actual_compiler_source = "env_only"
            scons_actual_compiler_install = normalize_string(
                vcvars_details.get("compiler_installation_root")
            )
    scons_compiler_matches_probe = None
    if recommended_toolchain_cl and scons_actual_compiler_path:
        scons_compiler_matches_probe = (
            pathlib.Path(recommended_toolchain_cl) == pathlib.Path(scons_actual_compiler_path)
        )

    next_action = None
    next_action_reason = None
    next_command = None
    if launcher is None:
        next_action = "install_or_enable_scons"
        next_action_reason = (
            "SCons unavailable; run the Godot SCons build only after SCons is installed or enabled."
        )
        next_command = recommended_probe
    elif not msvc_ready:
        next_action = "activate_msvc_toolchain"
        next_action_reason = "MSVC `cl` must be available directly or through vcvarsall.bat."
        next_command = recommended_dev_shell or recommended_probe
    elif not d3d12_deps_ready and not accesskit_ready:
        next_action = "install_workspace_local_godot_build_deps"
        next_action_reason = (
            "The default Godot SCons build requires both workspace-local D3D12 Mesa/NIR deps and AccessKit "
            "under `target/grx/localappdata/Godot/build_deps`."
        )
        next_command = recommended_all_build_deps_install
    elif not d3d12_deps_ready:
        next_action = "install_workspace_local_d3d12_deps"
        next_action_reason = (
            "Godot `d3d12=yes` requires workspace-local Mesa/NIR deps under "
            "`target/grx/localappdata/Godot/build_deps`."
        )
        next_command = recommended_d3d12_install
    elif not accesskit_ready:
        next_action = "install_workspace_local_accesskit_deps"
        next_action_reason = (
            "The default Godot SCons build requires a workspace-local AccessKit SDK under "
            "`target/grx/localappdata/Godot/build_deps/accesskit`."
        )
        next_command = recommended_accesskit_install
    elif not build_ready and blockers:
        next_action = "resolve_remaining_build_blockers"
        next_action_reason = blockers[0]
        next_command = recommended_probe
    elif isinstance(build_summary, dict) and not build_summary_required_args_satisfied:
        next_action = "rebuild_godot_with_path_overrides"
        next_action_reason = (
            "Existing Godot build summary does not prove `disable_path_overrides=no`; "
            "rebuild before running fresh load smoke."
        )
        next_command = r"py -3 ci\godot_rurix_scons_build.py"
    elif build_artifacts_ready and load_smoke_ready:
        if scenes_ready and runner_ready:
            if grx006_ready:
                if not grx007_ready:
                    next_action = "start_grx007_visual_diff_scaffold"
                    next_action_reason = (
                        "GRX-006 baseline/perf schema and strict perf gate format "
                        "infrastructure is available and parseable; proceed to GRX-007 "
                        "visual capture / diff scaffold."
                    )
                    next_command = (
                        r"py -3 spike\godot-rurix\bench\visual_diff.py --validate-only "
                        r"spike\godot-rurix\bench\samples\visual_diff_placeholder.json"
                    )
                elif not grx008_ready:
                    next_action = "start_grx008_fallback_telemetry_scaffold"
                    next_action_reason = (
                        "GRX-007 visual diff scaffold/hardening red/green samples pass; "
                        "proceed to GRX-008 fallback telemetry scaffold."
                    )
                    next_command = (
                        r"py -3 spike\godot-rurix\bench\fallback_telemetry.py --validate-only "
                        r"spike\godot-rurix\bench\samples\fallback_telemetry_placeholder.json"
                    )
                else:
                    if not grx009_ready:
                        next_action = "start_grx009_luminance_reduction_pass_contract"
                        next_action_reason = (
                            "GRX-007 visual diff and GRX-008 fallback telemetry "
                            "scaffold/hardening red/green samples all pass; produce the "
                            "GRX-009 luminance reduction pass contract and manifest under "
                            "spike/godot-rurix/passes/luminance_reduction. This is "
                            "preparation only: no actual Rurix acceleration pass, visual "
                            "verification, fallback wiring, or performance improvement is "
                            "implemented or claimed."
                        )
                        next_command = None
                    else:
                        if grx009_segment3a:
                            next_action = (
                                "start_grx009_luminance_segment3_resource_mapping"
                            )
                            next_action_reason = (
                                "GRX-009 segment 3a offline compile evidence is ready: "
                                "the manifest and compile evidence agree on success, "
                                "the DXIL/root signature/descriptor layout artifacts all "
                                "exist, and runtime still remains fallback-only. Proceed "
                                "to segment 3 resource mapping and gated runtime wiring."
                            )
                        elif grx009_segment2:
                            if grx009_compile_status in {
                                "compile_failed",
                                "toolchain_missing",
                            }:
                                next_action = "fix_grx009_luminance_segment3a_dxil_container_body_lowering_blocker"
                                next_action_reason = (
                                    "GRX-009 segment 2 core call-site fallback wiring is in "
                                    "place, but the latest segment 3a offline compile attempt "
                                    f"recorded {grx009_compile_status} evidence. Keep runtime fallback "
                                    "active, keep the manifest at segment 2, and resolve the "
                                    f"DXIL container/body lowering blocker first: {grx009_compile_blocker_summary or 'see offline_compile_evidence.json'}."
                                )
                            elif grx009_compile_status == "success":
                                next_action = (
                                    "fix_grx009_luminance_compile_artifact_gaps"
                                )
                                next_action_reason = (
                                    "A GRX-009 segment 3a compile attempt reported success, "
                                    "but the manifest/evidence gate is still not ready. Do "
                                    "not advance past segment 2 until DXIL, root signature, "
                                    "and descriptor layout artifacts are all present and "
                                    "traceable."
                                )
                            elif (
                                grx009_patch_stack_state is not None
                                and grx009_patch_stack_state != "0001+0002+0003"
                            ):
                                next_action = "restore_grx009_patch_stack_segment2"
                                next_action_reason = (
                                    "GRX-009 segment 2 artifacts exist, but the ignored "
                                    "Godot snapshot patch stack is not at the required "
                                    "`0001+0002+0003` state. Re-establish the legal patch "
                                    "stack before advancing to segment 3a compile work."
                                )
                            elif not grx009_patch_stack_is_ready:
                                next_action = "restore_grx009_patch_stack_segment2"
                                next_action_reason = (
                                    "GRX-009 segment 2 requires the shared patch stack check "
                                    "to pass before offline compile evidence work can be "
                                    "trusted. Fix the ignored Godot snapshot drift first."
                                )
                            else:
                                next_action = "start_grx009_luminance_reduction_real_gpu_pass"
                                next_action_reason = (
                                    "GRX-009 segment 2 core call-site fallback wiring is in "
                                    "place (patch 0003), but the actual Rurix GPU luminance "
                                    "pass is still NOT implemented. The per-pass setting "
                                    "still defaults to disabled, the bridge still returns "
                                    "fallback for luminance_reduction, and no performance "
                                    "improvement or visual verification is claimed yet."
                                )
                        elif grx009_compile_status in {"compile_failed", "toolchain_missing"}:
                            next_action = "restore_grx009_segment2_then_fix_compile_blocker"
                            next_action_reason = (
                                f"A GRX-009 {grx009_compile_status} evidence document exists, but the "
                                "segment 2 gate is not currently coherent. Re-establish the "
                                "segment 2 wiring gate first, then continue fixing the "
                                "offline compile blocker."
                            )
                        elif grx009_segment1:
                            next_action = (
                                "start_grx009_luminance_core_callsite_fallback_wiring"
                            )
                            next_action_reason = (
                                "GRX-009 segment 1 gated scaffold is delivered: the "
                                "bridge gate, 0002 module patch, and disabled/fallback "
                                "sample are all present, but the Godot core Auto Exposure "
                                "call site is not wired yet. Proceed to segment 2 core "
                                "call-site fallback wiring."
                            )
                        else:
                            next_action = (
                                "start_grx009_luminance_reduction_pass_gated_scaffold_segment1"
                            )
                            next_action_reason = (
                                "GRX-009 preparation is in place, but the segment 1 gated "
                                "scaffold evidence is incomplete. Re-establish the bridge "
                                "gate, 0002 module patch markers, and disabled/fallback "
                                "sample before advancing."
                            )
                        next_command = None
            else:
                next_action = "start_grx006_baseline_schema_perf_gate"
                next_action_reason = (
                    "GRX-001~005 build/load/scenes/runner evidence is complete; proceed to "
                    "GRX-006 baseline schema / perf gate input format."
                )
                next_command = (
                    r"py -3 spike\godot-rurix\bench\perf_gate.py --kind baseline "
                    r"--validate-only spike\godot-rurix\bench\samples\baseline_smoke_example.json"
                )
        elif scenes_ready:
            next_action = "run_grx005_benchmark_runner"
            next_action_reason = (
                "GRX-004 per-scene smoke is complete, but no GRX-005 runner evidence is "
                "present; run the benchmark runner."
            )
            next_command = (
                r"py -3 spike\godot-rurix\bench\run_benchmark_scenes.py --quick-smoke"
            )
        else:
            next_action = "start_grx2_tier0_benchmark_skeleton"
            next_action_reason = (
                "GRX-001/002/003 build/load/fallback evidence is complete; proceed to GRX-004."
            )
    elif build_artifacts_ready:
        next_action = "run_grx003_load_smoke"
        next_action_reason = (
            "Godot build summary is success and required artifacts are present; proceed to GRX-003 load smoke."
        )
        next_command = LOAD_SMOKE_COMMAND
    elif build_ready:
        next_action = "run_godot_scons_build"
        next_action_reason = "All required blockers are clear for the default `d3d12=yes` build."
        next_command = recommended_scons

    return {
        "build_ready": build_ready,
        "build_artifacts_ready": build_artifacts_ready,
        "load_smoke_ready": load_smoke_ready,
        "bench_scenes_ready": scenes_ready,
        "bench_runner_ready": runner_ready,
        "grx006_schema_ready": grx006_ready,
        "grx007_visual_ready": grx007_ready,
        "grx008_telemetry_ready": grx008_ready,
        "grx009_prep_ready": grx009_ready,
        "grx009_segment1_ready": grx009_segment1,
        "grx009_segment2_ready": grx009_segment2,
        "grx009_patch_stack_state": grx009_patch_stack_state,
        "grx009_patch_stack_ready": grx009_patch_stack_is_ready,
        "grx009_patch_stack_reason": grx009_patch_stack_reason,
        "grx009_segment3a_compile_ready": grx009_segment3a,
        "grx009_compile_evidence_status": grx009_compile_status,
        "grx009_compile_evidence_path": (
            str(GRX009_COMPILE_EVIDENCE) if GRX009_COMPILE_EVIDENCE.exists() else None
        ),
        "grx009_compile_blocker_category": grx009_compile_blocker_category,
        "grx009_compile_blocker_summary": grx009_compile_blocker_summary,
        "workspace_localappdata": str(LOCAL_GODOT_LOCALAPPDATA),
        "godot_build_deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "godot_windows_arch": GODOT_WINDOWS_ARCH,
        "scons_source": scons_source,
        "preferred_scons_launcher": launcher,
        "recommended_toolchain_cl": recommended_toolchain_cl,
        "recommended_toolchain_install": recommended_toolchain_install,
        "scons_actual_compiler_path": scons_actual_compiler_path,
        "scons_actual_compiler_source": scons_actual_compiler_source,
        "scons_actual_compiler_install": scons_actual_compiler_install,
        "scons_compiler_matches_probe": scons_compiler_matches_probe,
        "build_summary_command": build_summary_primary_cmd,
        "build_summary_ice_workaround_command": build_summary_ice_cmd,
        "build_summary_status": build_summary_status,
        "build_summary_required_scons_args": list(REQUIRED_SCONS_ARGS),
        "build_summary_required_scons_args_satisfied": build_summary_required_args_satisfied,
        "build_summary_path_overrides_ready": build_summary_required_args_satisfied,
        "last_build_summary_path": str(BUILD_SUMMARY_REPORT) if BUILD_SUMMARY_REPORT.exists() else None,
        "last_load_smoke_summary_path": (
            str(LOAD_SMOKE_SUMMARY_REPORT) if LOAD_SMOKE_SUMMARY_REPORT.exists() else None
        ),
        "recommended_probe_command": recommended_probe,
        "recommended_scons_command": recommended_scons,
        "ice_workaround_command": ice_workaround_command,
        "recommended_accesskit_install_command": recommended_accesskit_install,
        "recommended_d3d12_install_command": recommended_d3d12_install,
        "recommended_dev_shell_command": recommended_dev_shell,
        "next_action": next_action,
        "next_action_reason": next_action_reason,
        "next_command": next_command,
        "blockers": blockers,
        "warnings": warnings,
        "optional_tools_missing": optional_tools_missing,
        "results": by_name,
    }


def write_report(summary: dict[str, object]) -> None:
    LOCAL_LOG_DIR.mkdir(parents=True, exist_ok=True)
    JSON_REPORT.write_text(
        json.dumps(summary, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
    )
    print(f"[godot-toolchain] report_path: {JSON_REPORT}")


def main() -> int:
    results: list[ProbeResult] = []
    results.extend(probe_godot_tree())
    results.append(
        ProbeResult(
            "godot_workspace_localappdata",
            "PASS",
            "workspace-local LOCALAPPDATA root is configured",
            {
                "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
                "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
                "arch": GODOT_WINDOWS_ARCH,
            },
        )
    )
    results.append(run_probe("scons_cli", ["scons", "--version"]))
    results.append(run_probe("python_scons", ["py", "-3", "-m", "SCons", "--version"]))
    results.append(
        run_probe(
            "local_python_scons",
            [str(LOCAL_SCONS_PYTHON), "-m", "SCons", "--version"],
        )
    )
    vs_probe = probe_vs_build_tools()
    results.append(vs_probe)
    results.append(probe_msvc(vs_probe))
    results.append(probe_msvc_via_vcvarsall(vs_probe))
    results.append(probe_headers())
    results.append(probe_godot_accesskit_deps())
    results.append(probe_godot_d3d12_deps())
    results.append(probe_godot_agility_sdk())
    results.append(probe_godot_pix_runtime())
    for tool_name in TOOL_CANDIDATES:
        results.append(probe_tool_path(tool_name))
    results.append(probe_rurix_godot_dll())

    summary = summarize(results)
    for result in results:
        print_result(result)
    print(
        "[godot-toolchain] build_ready: "
        + ("PASS" if summary["build_ready"] else "SKIP")
        + f" - preferred launcher: {summary['preferred_scons_launcher'] or 'none'}"
    )
    print(
        "[godot-toolchain] load_smoke_ready: "
        + ("true" if summary["load_smoke_ready"] else "false")
    )
    print(
        "[godot-toolchain] bench_scenes_ready: "
        + ("true" if summary["bench_scenes_ready"] else "false")
    )
    print(
        "[godot-toolchain] bench_runner_ready: "
        + ("true" if summary["bench_runner_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx006_schema_ready: "
        + ("true" if summary["grx006_schema_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx007_visual_ready: "
        + ("true" if summary["grx007_visual_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx008_telemetry_ready: "
        + ("true" if summary["grx008_telemetry_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_prep_ready: "
        + ("true" if summary["grx009_prep_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_segment1_ready: "
        + ("true" if summary["grx009_segment1_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_segment2_ready: "
        + ("true" if summary["grx009_segment2_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_patch_stack_state: "
        + str(summary["grx009_patch_stack_state"] or "unknown")
    )
    print(
        "[godot-toolchain] grx009_patch_stack_ready: "
        + ("true" if summary["grx009_patch_stack_ready"] else "false")
    )
    if summary["grx009_patch_stack_reason"]:
        print(
            "[godot-toolchain] grx009_patch_stack_reason: "
            + str(summary["grx009_patch_stack_reason"])
        )
    print(
        "[godot-toolchain] grx009_segment3a_compile_ready: "
        + ("true" if summary["grx009_segment3a_compile_ready"] else "false")
    )
    if summary["grx009_compile_evidence_status"]:
        print(
            "[godot-toolchain] grx009_compile_evidence_status: "
            + str(summary["grx009_compile_evidence_status"])
        )
    if summary["grx009_compile_evidence_path"]:
        print(
            "[godot-toolchain] grx009_compile_evidence_path: "
            + str(summary["grx009_compile_evidence_path"])
        )
    if summary["grx009_compile_blocker_category"]:
        print(
            "[godot-toolchain] grx009_compile_blocker_category: "
            + str(summary["grx009_compile_blocker_category"])
        )
    if summary["grx009_compile_blocker_summary"]:
        print(
            "[godot-toolchain] grx009_compile_blocker_summary: "
            + str(summary["grx009_compile_blocker_summary"])
        )
    print("[godot-toolchain] scons_source: " + str(summary["scons_source"]))
    if summary["recommended_toolchain_cl"]:
        print(
            "[godot-toolchain] recommended_toolchain_cl: "
            + str(summary["recommended_toolchain_cl"])
        )
    if summary["recommended_toolchain_install"]:
        print(
            "[godot-toolchain] recommended_toolchain_install: "
            + str(summary["recommended_toolchain_install"])
        )
    if summary["scons_actual_compiler_path"]:
        print(
            "[godot-toolchain] scons_actual_compiler_path: "
            + str(summary["scons_actual_compiler_path"])
        )
    if summary["scons_actual_compiler_source"]:
        print(
            "[godot-toolchain] scons_actual_compiler_source: "
            + str(summary["scons_actual_compiler_source"])
        )
    if summary["scons_compiler_matches_probe"] is not None:
        print(
            "[godot-toolchain] scons_compiler_matches_probe: "
            + ("PASS" if summary["scons_compiler_matches_probe"] else "MISMATCH")
        )
    print(
        "[godot-toolchain] build_summary_required_scons_args_satisfied: "
        + ("true" if summary["build_summary_required_scons_args_satisfied"] else "false")
    )
    if summary["build_summary_command"]:
        print(
            "[godot-toolchain] build_summary_command: "
            + str(summary["build_summary_command"])
        )
    if summary["build_summary_ice_workaround_command"]:
        print(
            "[godot-toolchain] build_summary_ice_workaround_command: "
            + str(summary["build_summary_ice_workaround_command"])
        )
    if summary["last_load_smoke_summary_path"]:
        print(
            "[godot-toolchain] last_load_smoke_summary_path: "
            + str(summary["last_load_smoke_summary_path"])
        )
    for blocker in summary["blockers"]:
        print(f"[godot-toolchain] blocker: {blocker}")
    for warning in summary["warnings"]:
        print(f"[godot-toolchain] warning: {warning}")
    for optional_tool in summary["optional_tools_missing"]:
        print(f"[godot-toolchain] optional_tool_missing: {optional_tool}")
    print(
        "[godot-toolchain] recommended_probe_command: "
        + summary["recommended_probe_command"]
    )
    print(
        "[godot-toolchain] recommended_scons_command: "
        + str(summary["recommended_scons_command"])
    )
    print(
        "[godot-toolchain] ice_workaround_command: "
        + str(summary["ice_workaround_command"])
    )
    print(
        "[godot-toolchain] recommended_accesskit_install_command: "
        + str(summary["recommended_accesskit_install_command"])
    )
    print(
        "[godot-toolchain] recommended_d3d12_install_command: "
        + str(summary["recommended_d3d12_install_command"])
    )
    if summary["recommended_dev_shell_command"]:
        print(
            "[godot-toolchain] recommended_dev_shell_command: "
            + summary["recommended_dev_shell_command"]
        )
    if summary["next_action"]:
        print("[godot-toolchain] next_action: " + summary["next_action"])
    if summary["next_action_reason"]:
        print("[godot-toolchain] next_action_reason: " + summary["next_action_reason"])
    if summary["next_command"]:
        print("[godot-toolchain] next_command: " + str(summary["next_command"]))
    write_report(summary)

    if any(result.status == "FAIL" for result in results):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
