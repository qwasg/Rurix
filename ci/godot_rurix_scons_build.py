#!/usr/bin/env python3
"""Run the GRX Godot SCons build with reproducible toolchain evidence."""

from __future__ import annotations

import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
import hashlib


ROOT = pathlib.Path(__file__).resolve().parents[1]
EXTERNAL_GODOT = ROOT / "external" / "godot-master"
LOCAL_LOG_DIR = ROOT / "target" / "grx"
LOG_PATH = LOCAL_LOG_DIR / "godot_scons_build.log"
SUMMARY_PATH = LOCAL_LOG_DIR / "godot_scons_build_summary.json"
PROBE_REPORT = LOCAL_LOG_DIR / "godot_toolchain_probe.json"
LOCAL_SCONS_PYTHON = LOCAL_LOG_DIR / "scons-venv" / "Scripts" / "python.exe"
LOCAL_GODOT_LOCALAPPDATA = LOCAL_LOG_DIR / "localappdata"
VSWHERE = pathlib.Path(
    os.environ.get(
        "RURIX_VSWHERE",
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    )
)
VCVARSALL_REL = pathlib.Path("VC") / "Auxiliary" / "Build" / "vcvarsall.bat"
SCONS_BASE_ARGS = [
    "platform=windows",
    "target=template_debug",
    "d3d12=yes",
    "module_rurix_accel_enabled=yes",
    "disable_path_overrides=no",
]
SCONS_ICE_ARGS = ["num_jobs=1", "verbose=yes", "angle=no", "silence_msvc=no"]
REQUIRED_SCONS_ARGS = ("disable_path_overrides=no",)
TOOLCHAIN_ENV_KEYS = (
    "VSINSTALLDIR",
    "VCINSTALLDIR",
    "VCToolsInstallDir",
    "VCTOOLSINSTALLDIR",
    "VisualStudioVersion",
)
VS_INSTALL_RE = re.compile(
    r"(?i)([A-Z]:\\[^:\n\r]*?Microsoft Visual Studio\\\d{4}\\[^\\\n\r]+)"
)
COMPILER_PATH_RE = re.compile(
    r'([A-Z]:\\[^"\r\n]+?(?:clang-cl(?:\.exe)?|cl\.exe))',
    re.IGNORECASE,
)
FAIL_TARGET_RE = re.compile(r"scons:\s+\*\*\*\s+\[([^\]]+)\]")
REQUIRED_ARTIFACTS = {
    "godot_exe": EXTERNAL_GODOT / "bin" / "godot.windows.template_debug.x86_64.exe",
    "godot_console_exe": EXTERNAL_GODOT
    / "bin"
    / "godot.windows.template_debug.x86_64.console.exe",
    "module_rurix_accel_lib": EXTERNAL_GODOT
    / "bin"
    / "obj"
    / "modules"
    / "module_rurix_accel.windows.template_debug.x86_64.lib",
}


def cleaned_lines(text: str) -> list[str]:
    return [line.strip() for line in text.splitlines() if line.strip()]


def format_mtime_utc(timestamp: float) -> str:
    return (
        datetime.fromtimestamp(timestamp, tz=timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def completed_output(proc: subprocess.CompletedProcess[str]) -> str:
    parts = []
    stdout = (proc.stdout or "").strip()
    stderr = (proc.stderr or "").strip()
    if stdout:
        parts.append(stdout)
    if stderr:
        parts.append(stderr)
    return "\n".join(parts)


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


def run_cmd(command: str, *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cmd.exe", "/d", "/c", command],
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )


def run_process(
    args: list[str],
    *,
    cwd: pathlib.Path,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=cwd,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )


def load_json(path: pathlib.Path) -> dict[str, object] | None:
    if not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def collect_artifacts() -> dict[str, dict[str, object]]:
    artifacts: dict[str, dict[str, object]] = {}
    for name, path in REQUIRED_ARTIFACTS.items():
        artifact: dict[str, object] = {
            "path": str(path),
            "exists": path.exists(),
            "size_bytes": None,
            "mtime_utc": None,
            "sha256": None,
        }
        if path.exists():
            try:
                stats = path.stat()
                artifact["size_bytes"] = stats.st_size
                artifact["mtime_utc"] = format_mtime_utc(stats.st_mtime)
                artifact["sha256"] = sha256_file(path)
            except OSError:
                pass
        artifacts[name] = artifact
    return artifacts


def newest_subdir(root: pathlib.Path) -> pathlib.Path | None:
    if not root.exists():
        return None
    subdirs = sorted(path for path in root.iterdir() if path.is_dir())
    return subdirs[-1] if subdirs else None


def render_command(parts: list[str]) -> str:
    return subprocess.list2cmdline(parts)


def command_has_required_scons_args(command: str | None) -> bool:
    if not command:
        return False
    return all(arg in command for arg in REQUIRED_SCONS_ARGS)


def write_log(text: str, *, append: bool) -> None:
    LOCAL_LOG_DIR.mkdir(parents=True, exist_ok=True)
    mode = "a" if append else "w"
    with LOG_PATH.open(mode, encoding="utf-8", newline="\n") as handle:
        handle.write(text)


def run_vcvars_bat(vcvarsall: str, command: str) -> subprocess.CompletedProcess[str]:
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


def discover_scons_launcher() -> tuple[list[str] | None, str]:
    if shutil.which("scons"):
        return ["scons"], "existing"

    py_scons = run_process(["py", "-3", "-m", "SCons", "--version"], cwd=ROOT)
    if py_scons.returncode == 0:
        return ["py", "-3", "-m", "SCons"], "existing"

    if LOCAL_SCONS_PYTHON.exists():
        local_scons = run_process(
            [str(LOCAL_SCONS_PYTHON), "-m", "SCons", "--version"],
            cwd=ROOT,
        )
        if local_scons.returncode == 0:
            return [str(LOCAL_SCONS_PYTHON), "-m", "SCons"], "workspace-local venv"
    return None, "unavailable"


def discover_vs_toolchain() -> dict[str, str] | None:
    probe_payload = load_json(PROBE_REPORT)
    if isinstance(probe_payload, dict):
        details = (
            probe_payload.get("results", {})
            .get("vs_build_tools", {})
            .get("details", {})
        )
        if isinstance(details, dict):
            selected_install = normalize_string(details.get("selected_installation_path")) or normalize_string(
                details.get("installation_path")
            )
            vcvarsall = normalize_string(details.get("vcvarsall_bat"))
            candidate_cl = normalize_string(details.get("selected_candidate_cl")) or normalize_string(
                details.get("candidate_cl")
            )
            msvc_toolset = normalize_string(details.get("msvc_toolset"))
            if selected_install and vcvarsall:
                return {
                    "selected_installation_path": selected_install,
                    "vcvarsall_bat": vcvarsall,
                    "candidate_cl": candidate_cl or "",
                    "msvc_toolset": msvc_toolset or "",
                }

    env_install = normalize_string(os.environ.get("VSINSTALLDIR"))
    if env_install:
        install_path = pathlib.Path(env_install)
        vcvarsall = install_path / VCVARSALL_REL
        if vcvarsall.exists():
            msvc_toolset = newest_subdir(install_path / "VC" / "Tools" / "MSVC")
            candidate_cl = ""
            if msvc_toolset:
                probe_cl = msvc_toolset / "bin" / "Hostx64" / "x64" / "cl.exe"
                if probe_cl.exists():
                    candidate_cl = str(probe_cl)
            return {
                "selected_installation_path": str(install_path),
                "vcvarsall_bat": str(vcvarsall),
                "candidate_cl": candidate_cl,
                "msvc_toolset": str(msvc_toolset) if msvc_toolset else "",
            }

    if VSWHERE.exists():
        proc = run_process(
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
        )
        if proc.returncode == 0:
            try:
                installs = json.loads(proc.stdout or "[]")
            except json.JSONDecodeError:
                installs = []
            for install in installs:
                if not isinstance(install, dict):
                    continue
                install_path_value = normalize_string(install.get("installationPath"))
                if not install_path_value:
                    continue
                install_path = pathlib.Path(install_path_value)
                vcvarsall = install_path / VCVARSALL_REL
                if not vcvarsall.exists():
                    continue
                msvc_toolset = newest_subdir(install_path / "VC" / "Tools" / "MSVC")
                candidate_cl = ""
                if msvc_toolset:
                    probe_cl = msvc_toolset / "bin" / "Hostx64" / "x64" / "cl.exe"
                    if probe_cl.exists():
                        candidate_cl = str(probe_cl)
                return {
                    "selected_installation_path": str(install_path),
                    "vcvarsall_bat": str(vcvarsall),
                    "candidate_cl": candidate_cl,
                    "msvc_toolset": str(msvc_toolset) if msvc_toolset else "",
                }
    return None


def capture_vcvars_environment(vcvarsall: str) -> dict[str, str]:
    proc = run_vcvars_bat(vcvarsall, "set")
    if proc.returncode != 0:
        raise RuntimeError(completed_output(proc) or "vcvarsall.bat failed")
    env = os.environ.copy()
    for line in proc.stdout.splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        env[key] = value
        if key.lower() == "path":
            env["PATH"] = value
            env["Path"] = value
    env["LOCALAPPDATA"] = str(LOCAL_GODOT_LOCALAPPDATA)
    return env


def extract_compiler_path(text: str) -> str | None:
    collapsed = text.replace("\r", "").replace("\n", "")
    for candidate_text in (text, collapsed):
        match = COMPILER_PATH_RE.search(candidate_text)
        if match:
            return match.group(1).strip('"')
    return None


def collect_toolchain_evidence(env: dict[str, str]) -> dict[str, object]:
    details: dict[str, object] = {}
    for key in TOOLCHAIN_ENV_KEYS:
        value = normalize_string(env.get(key)) or normalize_string(env.get(key.upper()))
        if value:
            details[key] = value

    where_cl_proc = run_cmd("where cl", env=env)
    where_cl = cleaned_lines(where_cl_proc.stdout)
    if where_cl:
        details["where_cl"] = where_cl
    else:
        details["where_cl_error"] = completed_output(where_cl_proc) or str(where_cl_proc.returncode)

    cl_bv_proc = run_cmd("cl /Bv", env=env)
    cl_bv_output = completed_output(cl_bv_proc)
    if cl_bv_output:
        details["cl_bv"] = cl_bv_output
    details["cl_bv_exit_code"] = cl_bv_proc.returncode

    compiler_path = extract_compiler_path(cl_bv_output or "")
    compiler_source = None
    if compiler_path:
        compiler_source = "cl_bv"
    elif where_cl:
        compiler_path = where_cl[0]
        compiler_source = "where_cl"

    if compiler_path:
        details["compiler_path"] = compiler_path
        details["compiler_source"] = compiler_source
        install_root = infer_vs_installation_root(compiler_path)
        if install_root:
            details["compiler_installation_root"] = install_root
    return details


def detect_llvm_tools(env: dict[str, str]) -> dict[str, str]:
    found: dict[str, str] = {}
    for tool_name in ("clang-cl", "lld-link", "llvm-lib"):
        proc = run_cmd(f"where {tool_name}", env=env)
        lines = cleaned_lines(proc.stdout)
        if lines:
            found[tool_name] = lines[0]
    return found


def extract_failure_targets(text: str) -> list[str]:
    targets: list[str] = []
    seen: set[str] = set()
    for match in FAIL_TARGET_RE.finditer(text):
        target = match.group(1).strip()
        if target and target not in seen:
            seen.add(target)
            targets.append(target)
    return targets


def classify_blocker(hit_c1001: bool, failure_targets: list[str]) -> str | None:
    if not hit_c1001:
        return None
    for target in failure_targets:
        if "modules\\rurix_accel" in target.lower().replace("/", "\\"):
            return "integration_regression"
    return "external_toolchain_blocker"


def make_attempt(
    name: str,
    launcher: list[str],
    base_env: dict[str, str],
    *,
    extra_env: dict[str, str] | None = None,
    extra_args: list[str] | None = None,
    skipped_reason: str | None = None,
) -> dict[str, object]:
    env = base_env.copy()
    if extra_env:
        env.update(extra_env)

    evidence = collect_toolchain_evidence(env)
    args = launcher + SCONS_BASE_ARGS + SCONS_ICE_ARGS + (extra_args or [])
    command = render_command(args)
    attempt: dict[str, object] = {
        "name": name,
        "command": command,
        "cwd": str(EXTERNAL_GODOT),
        "toolchain_env": {key: env.get(key, "") for key in TOOLCHAIN_ENV_KEYS if env.get(key)},
        "where_cl": evidence.get("where_cl"),
        "cl_bv": evidence.get("cl_bv"),
        "skipped_reason": skipped_reason,
    }
    if skipped_reason:
        attempt["skipped"] = True
        attempt["exit_code"] = None
        attempt["hit_c1001"] = False
        attempt["failure_targets"] = []
        if evidence.get("compiler_path"):
            attempt["actual_compiler_path"] = evidence["compiler_path"]
            attempt["actual_compiler_source"] = evidence.get("compiler_source")
            attempt["actual_compiler_install"] = evidence.get("compiler_installation_root")
        return attempt

    proc = run_process(args, cwd=EXTERNAL_GODOT, env=env)
    output = completed_output(proc)
    actual_compiler_path = extract_compiler_path(output or "")
    actual_compiler_source = None
    if actual_compiler_path:
        actual_compiler_source = "build_log"
    elif evidence.get("compiler_path"):
        actual_compiler_path = str(evidence["compiler_path"])
        actual_compiler_source = str(evidence.get("compiler_source") or "where_cl")
    failure_targets = extract_failure_targets(output)
    hit_c1001 = "C1001" in output

    attempt["exit_code"] = proc.returncode
    attempt["hit_c1001"] = hit_c1001
    attempt["failure_targets"] = failure_targets
    attempt["blocker_type"] = classify_blocker(hit_c1001, failure_targets)
    if actual_compiler_path:
        attempt["actual_compiler_path"] = actual_compiler_path
        attempt["actual_compiler_source"] = actual_compiler_source
        install_root = infer_vs_installation_root(actual_compiler_path)
        if install_root:
            attempt["actual_compiler_install"] = install_root
    attempt["output"] = output
    return attempt


def summarize_attempts(
    attempts: list[dict[str, object]],
    *,
    probe_candidate_cl: str | None,
    probe_candidate_install: str | None,
) -> dict[str, object]:
    executed_attempts = [attempt for attempt in attempts if not attempt.get("skipped")]
    artifacts = collect_artifacts()
    missing_artifacts = [
        name for name, artifact in artifacts.items() if not bool(artifact.get("exists"))
    ]
    artifacts_complete = all(
        bool(artifact.get("exists"))
        and artifact.get("size_bytes") is not None
        and artifact.get("mtime_utc") is not None
        and artifact.get("sha256") is not None
        for artifact in artifacts.values()
    )
    successful_attempt = next(
        (attempt for attempt in executed_attempts if attempt.get("exit_code") == 0),
        None,
    )
    final_attempt = executed_attempts[-1] if executed_attempts else attempts[-1]
    effective_attempt = successful_attempt or final_attempt
    actual_compiler_path = normalize_string(final_attempt.get("actual_compiler_path"))
    actual_compiler_source = normalize_string(final_attempt.get("actual_compiler_source"))
    actual_compiler_install = normalize_string(final_attempt.get("actual_compiler_install"))
    toolchain_mismatch = None
    if probe_candidate_cl and actual_compiler_path:
        toolchain_mismatch = pathlib.Path(probe_candidate_cl) != pathlib.Path(actual_compiler_path)

    blocker_type = normalize_string(final_attempt.get("blocker_type"))
    if not blocker_type and successful_attempt is None and any(
        attempt.get("hit_c1001") for attempt in executed_attempts
    ):
        blocker_type = "external_toolchain_blocker"
    if not blocker_type and successful_attempt is not None and not artifacts_complete:
        blocker_type = "incomplete_build_artifacts"

    stable_repro_command = None
    for attempt in reversed(executed_attempts):
        if attempt.get("hit_c1001"):
            stable_repro_command = normalize_string(attempt.get("command"))
            break
    if stable_repro_command is None and successful_attempt is None:
        stable_repro_command = normalize_string(final_attempt.get("command"))

    first_failing_sources = []
    seen_targets: set[str] = set()
    for attempt in executed_attempts:
        for target in attempt.get("failure_targets") or []:
            if target not in seen_targets:
                seen_targets.add(target)
                first_failing_sources.append(target)
        if first_failing_sources:
            break

    command = normalize_string(effective_attempt.get("command"))
    ice_workaround_command = normalize_string(attempts[0].get("command")) if attempts else None
    required_scons_args_satisfied = command_has_required_scons_args(
        command
    ) and command_has_required_scons_args(ice_workaround_command)

    return {
        "status": "success"
        if successful_attempt and artifacts_complete
        else "external_toolchain_blocker",
        "command": command,
        "attempts": [
            {
                key: value
                for key, value in attempt.items()
                if key != "output"
            }
            for attempt in attempts
        ],
        "actual_compiler_path": actual_compiler_path,
        "actual_compiler_source": actual_compiler_source,
        "actual_compiler_install": actual_compiler_install,
        "probe_candidate_compiler_path": probe_candidate_cl,
        "probe_candidate_install": probe_candidate_install,
        "toolchain_mismatch": toolchain_mismatch,
        "blocker_type": blocker_type,
        "first_failing_sources": first_failing_sources,
        "artifacts": artifacts,
        "artifacts_complete": artifacts_complete,
        "missing_artifacts": missing_artifacts,
        "required_scons_args": list(REQUIRED_SCONS_ARGS),
        "required_scons_args_satisfied": required_scons_args_satisfied,
        "path_overrides_ready": required_scons_args_satisfied,
        "workaround_for_msvc_c1001": True,
        "stable_repro_command": stable_repro_command,
        "ice_workaround_command": ice_workaround_command or "",
        "log_path": str(LOG_PATH),
    }


def main() -> int:
    launcher, scons_source = discover_scons_launcher()
    if launcher is None:
        raise SystemExit("No usable SCons launcher was found.")

    toolchain = discover_vs_toolchain()
    if not toolchain:
        raise SystemExit("No usable Visual Studio toolchain was found.")

    selected_install = toolchain["selected_installation_path"]
    vcvarsall = toolchain["vcvarsall_bat"]
    probe_candidate_cl = normalize_string(toolchain.get("candidate_cl"))
    probe_candidate_install = normalize_string(selected_install)
    msvc_toolset = normalize_string(toolchain.get("msvc_toolset"))
    msvc_version = pathlib.Path(msvc_toolset).name if msvc_toolset else None

    base_env = capture_vcvars_environment(vcvarsall)

    header = [
        "# GRX Godot SCons build log",
        f"scons_source: {scons_source}",
        f"selected_installation_path: {selected_install}",
        f"probe_candidate_compiler_path: {probe_candidate_cl or ''}",
        "",
    ]
    write_log("\n".join(header), append=False)

    attempts: list[dict[str, object]] = []
    attempt_a = make_attempt("attempt_a_msvc_single_job", launcher, base_env)
    attempts.append(attempt_a)

    for attempt in attempts:
        log_chunks = [
            f"===== {attempt['name']} =====",
            f"command: {attempt['command']}",
            f"cwd: {attempt['cwd']}",
        ]
        if attempt.get("toolchain_env"):
            log_chunks.append("toolchain_env:")
            for key, value in dict(attempt["toolchain_env"]).items():
                log_chunks.append(f"  {key}={value}")
        if attempt.get("where_cl"):
            log_chunks.append("where cl:")
            for line in attempt["where_cl"]:
                log_chunks.append(f"  {line}")
        if attempt.get("cl_bv"):
            log_chunks.append("cl /Bv:")
            log_chunks.append(str(attempt["cl_bv"]))
        if attempt.get("output"):
            log_chunks.append("output:")
            log_chunks.append(str(attempt["output"]))
        log_chunks.append("")
        write_log("\n".join(log_chunks), append=True)

    if attempt_a.get("exit_code") != 0 and attempt_a.get("hit_c1001"):
        fixed_env: dict[str, str] = {"VSINSTALLDIR": selected_install}
        vc_dir = str(pathlib.Path(selected_install) / "VC") + "\\"
        fixed_env["VCINSTALLDIR"] = vc_dir
        if msvc_toolset:
            fixed_env["VCToolsInstallDir"] = str(pathlib.Path(msvc_toolset)) + "\\"
            fixed_env["VCTOOLSINSTALLDIR"] = fixed_env["VCToolsInstallDir"]
        extra_args = [f"msvc_version={msvc_version}"] if msvc_version else []
        attempt_b = make_attempt(
            "attempt_b_pinned_msvc",
            launcher,
            base_env,
            extra_env=fixed_env,
            extra_args=extra_args,
        )
        attempts.append(attempt_b)
        log_chunks = [
            f"===== {attempt_b['name']} =====",
            f"command: {attempt_b['command']}",
            f"cwd: {attempt_b['cwd']}",
        ]
        if attempt_b.get("toolchain_env"):
            log_chunks.append("toolchain_env:")
            for key, value in dict(attempt_b["toolchain_env"]).items():
                log_chunks.append(f"  {key}={value}")
        if attempt_b.get("where_cl"):
            log_chunks.append("where cl:")
            for line in attempt_b["where_cl"]:
                log_chunks.append(f"  {line}")
        if attempt_b.get("cl_bv"):
            log_chunks.append("cl /Bv:")
            log_chunks.append(str(attempt_b["cl_bv"]))
        if attempt_b.get("output"):
            log_chunks.append("output:")
            log_chunks.append(str(attempt_b["output"]))
        log_chunks.append("")
        write_log("\n".join(log_chunks), append=True)

        if attempt_b.get("exit_code") != 0 and attempt_b.get("hit_c1001"):
            llvm_tools = detect_llvm_tools(base_env)
            if all(tool in llvm_tools for tool in ("clang-cl", "lld-link", "llvm-lib")):
                attempt_c = make_attempt(
                    "attempt_c_use_llvm",
                    launcher,
                    base_env,
                    extra_args=["use_llvm=yes"],
                )
            else:
                missing = [
                    tool_name
                    for tool_name in ("clang-cl", "lld-link", "llvm-lib")
                    if tool_name not in llvm_tools
                ]
                attempt_c = make_attempt(
                    "attempt_c_use_llvm",
                    launcher,
                    base_env,
                    extra_args=["use_llvm=yes"],
                    skipped_reason="missing LLVM tools: " + ", ".join(missing),
                )
                attempt_c["llvm_tools"] = llvm_tools
            attempts.append(attempt_c)
            log_chunks = [
                f"===== {attempt_c['name']} =====",
                f"command: {attempt_c['command']}",
                f"cwd: {attempt_c['cwd']}",
            ]
            if attempt_c.get("toolchain_env"):
                log_chunks.append("toolchain_env:")
                for key, value in dict(attempt_c["toolchain_env"]).items():
                    log_chunks.append(f"  {key}={value}")
            if attempt_c.get("where_cl"):
                log_chunks.append("where cl:")
                for line in attempt_c["where_cl"]:
                    log_chunks.append(f"  {line}")
            if attempt_c.get("cl_bv"):
                log_chunks.append("cl /Bv:")
                log_chunks.append(str(attempt_c["cl_bv"]))
            if attempt_c.get("llvm_tools"):
                log_chunks.append("llvm_tools:")
                for key, value in dict(attempt_c["llvm_tools"]).items():
                    log_chunks.append(f"  {key}={value}")
            if attempt_c.get("skipped_reason"):
                log_chunks.append(f"skipped_reason: {attempt_c['skipped_reason']}")
            if attempt_c.get("output"):
                log_chunks.append("output:")
                log_chunks.append(str(attempt_c["output"]))
            log_chunks.append("")
            write_log("\n".join(log_chunks), append=True)

    summary = summarize_attempts(
        attempts,
        probe_candidate_cl=probe_candidate_cl,
        probe_candidate_install=probe_candidate_install,
    )
    SUMMARY_PATH.write_text(
        json.dumps(summary, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, indent=2, ensure_ascii=True))
    return 0 if summary["status"] == "success" else 1


if __name__ == "__main__":
    raise SystemExit(main())
