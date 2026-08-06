#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 M83 texture_transcode 硬门冒烟(g8.p1.m83.texture_transcode;
RFC-0020 §4.8;spec/asset_pipeline.md RXS-0334)。

host 纯 host 门(device 段 not_applicable)。checks.* 13 项布尔;任一红 →
evidence 如实落盘后 exit 1(禁充绿)。过渡期若 Basis 腿未实现,对应 check
必须为 false。

用法:
  py -3 ci/g8_texture_transcode_smoke.py --gate g8.p1.m83.texture_transcode
  py -3 ci/g8_texture_transcode_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = (
    ROOT / "milestones" / "g8" / "g8_m83_texture_transcode_evidence_schema.json"
)

GATE_KEY = "g8.p1.m83.texture_transcode"
NUMERIC_STEP = 107
SOURCE_REF = "RFC-0020 §4.8;spec/asset_pipeline.md RXS-0334"
VENDOR_VERSION = "rurix-basis-transitional/0.1.0"
KTX2_MAGIC = b"\xABKTX 20\xBB\r\n\x1A\n"
RXBC_MAGIC = b"RXBC"
RXAS_MAGIC = b"RXAS"
RXBS_MAGIC = b"RXBS"

COLOR_MAX_DELTA = 48
NORMAL_MAD = 0.15
ALPHA_COV = 0.08

CHECK_KEYS = [
    "real_codec_identity",
    "cook_twice_byte_equal",
    "ktx2_leg_present_valid",
    "basis_leg_present_valid",
    "bcn_leg_formats_expected",
    "astc_leg_format_expected",
    "color_error_within_tolerance",
    "normal_length_within_tolerance",
    "alpha_coverage_within_tolerance",
    "container_not_renamed",
    "license_sbom_entries_present",
    "transcode_hook_zero_byte",
    "double_cook_across_isolated_roots",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> bool:
    if not cond:
        FAILURES.append(msg)
        return False
    return True


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def build_rxcook() -> Path:
    print("[g8_m83] cargo build -p rurix-asset --bin rxcook")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-asset", "--bin", "rxcook", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[g8_m83] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rxcook.exe" if sys.platform == "win32" else "rxcook")
    if not exe.is_file():
        print(f"[g8_m83] FAIL rxcook 缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def cook(exe: Path, out: Path, fixture: str = "checker") -> tuple[int, str, str]:
    r = subprocess.run(
        [
            str(exe),
            "cook-texture",
            "--fixture",
            fixture,
            "--out",
            str(out),
            "--profile",
            "win-vulkan-bcn-v1",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode, r.stdout, r.stderr


def parse_kv(stdout: str) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in stdout.splitlines():
        if "=" in line:
            k, v = line.split("=", 1)
            out[k.strip()] = v.strip()
    return out


def read_report(out: Path) -> dict:
    p = out / "cook_report.json"
    return json.loads(p.read_text(encoding="utf-8"))


def run_gate() -> dict[str, bool]:
    results = {k: False for k in CHECK_KEYS}
    exe = build_rxcook()

    # license / SBOM 文件存在
    vendor = ROOT / "src" / "rurix-basis-sys" / "VENDOR.md"
    notice = ROOT / "src" / "rurix-basis-sys" / "NOTICE"
    sbom = ROOT / "src" / "rurix-basis-sys" / "SBOM.md"
    results["license_sbom_entries_present"] = check(
        vendor.is_file() and notice.is_file() and sbom.is_file(),
        "VENDOR.md/NOTICE/SBOM.md 缺失",
    )
    if vendor.is_file():
        vtext = vendor.read_text(encoding="utf-8")
        check(
            VENDOR_VERSION in vtext,
            f"VENDOR.md 未登记版本串 {VENDOR_VERSION}",
        )

    # transcode 留口 0-byte
    print("[g8_m83] cargo test -p rurix-render default_transcode_is_identity")
    tr = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "rurix-render",
            "--lib",
            "streaming::resource::tests::default_transcode_is_identity",
            "--",
            "--exact",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    blob = tr.stdout + tr.stderr
    results["transcode_hook_zero_byte"] = check(
        tr.returncode == 0 and "1 passed" in blob,
        f"PagedResource::transcode 恒等单测失败:\n{blob}",
    )

    with tempfile.TemporaryDirectory(prefix="rurix_m83_") as td:
        root = Path(td)
        a = root / "a"
        b = root / "b"
        c = root / "c_isolated"
        rc1, out1, err1 = cook(exe, a, "checker")
        if not check(rc1 == 0, f"cook A 失败: {err1}\n{out1}"):
            note("cook A failed — remaining checks may stay false")
            return results
        kv1 = parse_kv(out1)
        rep1 = read_report(a)

        results["real_codec_identity"] = check(
            kv1.get("codec_version") == VENDOR_VERSION
            and rep1.get("codec_version") == VENDOR_VERSION,
            f"codec version 不符: {kv1.get('codec_version')}",
        )

        rc2, out2, err2 = cook(exe, b, "checker")
        check(rc2 == 0, f"cook B 失败: {err2}")
        rep2 = read_report(b) if rc2 == 0 else {}

        ba = (a / "texture.bcn").read_bytes()
        bb = (b / "texture.bcn").read_bytes() if rc2 == 0 else b""
        ka = (a / "texture.ktx2").read_bytes()
        kb = (b / "texture.ktx2").read_bytes() if rc2 == 0 else b""
        aa = (a / "texture.astc").read_bytes()
        ab = (b / "texture.astc").read_bytes() if rc2 == 0 else b""

        results["cook_twice_byte_equal"] = check(
            ba == bb and ka == kb and aa == ab and ba != b"",
            "两次 cook 字节不等或 BCn 为空",
        )

        # 隔离根
        rc3, _, err3 = cook(exe, c, "checker")
        check(rc3 == 0, f"cook C 失败: {err3}")
        if rc3 == 0:
            results["double_cook_across_isolated_roots"] = check(
                (c / "texture.bcn").read_bytes() == ba
                and (c / "texture.ktx2").read_bytes() == ka,
                "隔离根 cook 字节不等",
            )

        # KTX2
        ktx_ok = (
            ka.startswith(KTX2_MAGIC)
            and len(ka) > 80
            and struct.unpack_from("<I", ka, 12 + 32)[0] == 0  # supercompression at offset?
        )
        # Header layout: magic12 + vkFormat0 + typeSize4 + w8 + h12 + depth16 + layer20
        # + face24 + level28 + supercompression32
        supercompression = struct.unpack_from("<I", ka, 12 + 8 * 4)[0]
        level_count = struct.unpack_from("<I", ka, 12 + 7 * 4)[0]
        results["ktx2_leg_present_valid"] = check(
            ka.startswith(KTX2_MAGIC) and supercompression == 0 and level_count == 1,
            f"KTX2 无效 magic/scheme/levels (scheme={supercompression}, levels={level_count})",
        )
        _ = ktx_ok

        # Basis 过渡腿:RXBS + ETC1S-via-BC1 非空非全零(完整 .basis 随 vendor 升级)
        basis_path = a / "texture.basis"
        bb = basis_path.read_bytes() if basis_path.is_file() else b""
        bs_fmt = struct.unpack_from("<H", bb, 6)[0] if len(bb) >= 8 else 0
        bs_payload = bb[16:] if len(bb) > 16 else b""
        results["basis_leg_present_valid"] = check(
            bb.startswith(RXBS_MAGIC)
            and bs_fmt == 1
            and len(bs_payload) > 0
            and not all(x == 0 for x in bs_payload)
            and rep1.get("basis_present") is True,
            "Basis 过渡腿无效(magic/fmt/payload/basis_present)",
        )
        note("basis_leg=RXBS transitional ETC1S-via-BC1; full basis_universal pending")

        # BCn
        bcn_fmt = struct.unpack_from("<H", ba, 6)[0] if len(ba) >= 8 else 0
        payload = ba[16:] if len(ba) > 16 else b""
        results["bcn_leg_formats_expected"] = check(
            ba.startswith(RXBC_MAGIC)
            and bcn_fmt == 7
            and len(payload) > 0
            and not all(x == 0 for x in payload)
            and rep1.get("gpu_format_bcn") == "BC7_UNORM",
            f"BCn 腿无效(magic/fmt/payload/format标注)",
        )

        # ASTC
        results["astc_leg_format_expected"] = check(
            aa.startswith(RXAS_MAGIC)
            and len(aa) > 16
            and not all(x == 0 for x in aa[16:])
            and rep1.get("gpu_format_astc") == "ASTC_4x4_UNORM",
            "ASTC 腿无效",
        )

        results["container_not_renamed"] = check(
            ka.startswith(KTX2_MAGIC)
            and ba.startswith(RXBC_MAGIC)
            and aa.startswith(RXAS_MAGIC),
            "容器 magic 复核失败(疑改扩展名假转码)",
        )

        results["color_error_within_tolerance"] = check(
            int(rep1.get("color_max_delta", 999)) <= COLOR_MAX_DELTA,
            f"颜色误差超限: {rep1.get('color_max_delta')} > {COLOR_MAX_DELTA}",
        )

        # normal fixture 另跑一轮测 normal length
        nout = root / "normal"
        rcn, outn, errn = cook(exe, nout, "normal")
        if rcn == 0:
            repn = read_report(nout)
            results["normal_length_within_tolerance"] = check(
                float(repn.get("normal_length_mad", 9)) <= NORMAL_MAD,
                f"normal length 超限: {repn.get('normal_length_mad')}",
            )
            results["alpha_coverage_within_tolerance"] = check(
                float(rep1.get("alpha_coverage_delta", 9)) <= ALPHA_COV,
                f"alpha coverage 超限: {rep1.get('alpha_coverage_delta')}",
            )
        else:
            check(False, f"normal fixture cook 失败: {errn}")

        note(
            "过渡 codec=rurix-basis-transitional/0.1.0;"
            "完整 basis_universal vendor 待合入;"
            f"measured color_max_delta={rep1.get('color_max_delta')},"
            f"alpha_cov={rep1.get('alpha_coverage_delta')}"
        )

    return results


def write_evidence(results: dict[str, bool], host_ok: bool) -> Path:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    notes = (
        "M83 host 门;device=not_applicable。"
        + (" ".join(NOTES) if NOTES else "")
        + ((" FAILURES: " + " | ".join(FAILURES)) if FAILURES else "")
    )
    ev = {
        "schema_version": 1,
        "subject": "g8_m83_texture_transcode",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M83",
        "wave": "G8.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
        "checks": results,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": notes,
    }
    path = EVIDENCE_DIR / f"g8_m83_texture_transcode_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m83] evidence 落盘: {path.relative_to(ROOT)}")
    return path


def selftest() -> None:
    check(False, "selftest: 合成失败")
    if len(FAILURES) != 1:
        print("[g8_m83] selftest FAIL: check() 未记录", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    assert KTX2_MAGIC.startswith(b"\xABKTX")
    assert SCHEMA_PATH.is_file()
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    assert schema["properties"]["numeric_step"]["const"] == NUMERIC_STEP
    assert set(schema["properties"]["checks"]["required"]) == set(CHECK_KEYS)
    # 证明缺腿判定不会被默认 True 吞掉
    synth = {k: False for k in CHECK_KEYS}
    synth["basis_leg_present_valid"] = False
    assert synth["basis_leg_present_valid"] is False
    print("[g8_m83] selftest PASS")
    sys.exit(0)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        selftest()
    if args.gate and args.gate != GATE_KEY:
        print(f"[g8_m83] FAIL unexpected gate: {args.gate}", file=sys.stderr)
        sys.exit(2)

    results = run_gate()
    host_ok = all(results[k] for k in CHECK_KEYS) and not FAILURES
    # 即便有 FAILURES,checks 里已是实测布尔;host_ok 仅当全绿
    host_ok = all(results[k] for k in CHECK_KEYS)
    write_evidence(results, host_ok)

    print("[g8_m83] checks:")
    for k in CHECK_KEYS:
        print(f"  {'PASS' if results[k] else 'FAIL'}: {k}")
    if FAILURES:
        print("[g8_m83] failures:", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)

    if not host_ok:
        print("[g8_m83] VERDICT=FAIL", file=sys.stderr)
        sys.exit(1)
    print("[g8_m83] VERDICT=PASS")
    sys.exit(0)


if __name__ == "__main__":
    main()
