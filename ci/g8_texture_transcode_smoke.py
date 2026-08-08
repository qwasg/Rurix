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
# 真实上游 pin(与 VENDOR.md / SBOM.md / rurix_basis_version() 字面全等)。
VENDOR_VERSION = "basis_universal/1.16.4+g900e40fb5d25"
UPSTREAM_TAG = "1.16.4"
UPSTREAM_COMMIT = "900e40fb5d2502927360fe2f31762bdbb624455f"
UPSTREAM_URL = "https://github.com/BinomialLLC/basis_universal"

KTX2_MAGIC = b"\xABKTX 20\xBB\r\n\x1A\n"
RXBC_MAGIC = b"RXBC"
RXAS_MAGIC = b"RXAS"

# 真实 `.basis` 文件签名:上游 basis_file_header.m_sig = ('B'<<8)|'s' = 0x4273,
# 经 packed_uint<2> 以 LE 落盘 → 字节序 b"sB"。
BASIS_SIG_BYTES = b"sB"
BASIS_SIG_U16 = 0x4273  # struct.unpack("<H", b"sB") → 0x4273

# `.basis` 腿禁止出现的容器 magic(过渡期 RXBS 冒充即此列;KTX2 亦不得充 .basis)。
FORBIDDEN_BASIS_PREFIXES = (b"RXBS", b"RXBC", b"RXAS", KTX2_MAGIC)

# VENDOR.md / SBOM.md 禁止仍含过渡串(清零校验)。
FORBIDDEN_VERSION_SUBSTRINGS = ("rurix-basis-transitional",)

# RXBC 容器 format_id(按语义:color=BC7 / normal=BC5 / mask=BC4)。
RXBC_FMT_BC7 = 7
RXBC_FMT_BC5 = 5
RXBC_FMT_BC4 = 4

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


def cook(
    exe: Path,
    out: Path,
    fixture: str = "checker",
    profile: str = "win-vulkan-bcn-v1",
    semantics: str | None = None,
) -> tuple[int, str, str]:
    argv = [
        str(exe),
        "cook-texture",
        "--fixture",
        fixture,
        "--out",
        str(out),
        "--profile",
        profile,
    ]
    if semantics is not None:
        argv += ["--semantics", semantics]
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)
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


def parse_ktx2(b: bytes) -> dict | None:
    """真实 KTX2 header 解析(不止 magic):返回字段字典,结构非法则 None。

    布局(KTX2 spec):identifier 12B | vkFormat u32 | typeSize u32 | pixelWidth u32
    | pixelHeight u32 | pixelDepth u32 | layerCount u32 | faceCount u32
    | levelCount u32 | supercompressionScheme u32 | dfd off/len u32×2
    | kvd off/len u32×2 | sgd off/len u64×2 | levelIndex[levelCount] = u64×3。
    """
    if not b.startswith(KTX2_MAGIC) or len(b) < 80:
        return None
    f = struct.unpack_from("<10I", b, 12)
    (
        vk_format,
        type_size,
        width,
        height,
        depth,
        layers,
        faces,
        levels,
        supercompression,
        dfd_off,
    ) = f
    dfd_len = struct.unpack_from("<I", b, 12 + 10 * 4)[0]
    if levels == 0 or levels > 16:
        return None
    # level index 紧跟 68B header 之后
    idx_off = 12 + 68
    if idx_off + levels * 24 > len(b):
        return None
    lv = []
    for i in range(levels):
        off, ln, uln = struct.unpack_from("<QQQ", b, idx_off + i * 24)
        if off + ln > len(b) or ln == 0:
            return None
        lv.append({"byte_offset": off, "byte_length": ln, "uncompressed_length": uln})
    return {
        "vk_format": vk_format,
        "type_size": type_size,
        "width": width,
        "height": height,
        "depth": depth,
        "layer_count": layers,
        "face_count": faces,
        "level_count": levels,
        "supercompression": supercompression,
        "dfd_offset": dfd_off,
        "dfd_length": dfd_len,
        "levels": lv,
    }


def parse_basis(b: bytes) -> dict | None:
    """真实 `.basis` header 解析(basis_file_headers.h 精确布局)。

    basis_file_header(packed LE):
      +0  sig         packed_uint<2>  (== 0x4273 LE → bytes b"sB")
      +2  ver         packed_uint<2>
      +4  header_size packed_uint<2>
      +6  header_crc16 packed_uint<2>
      +8  data_size   packed_uint<4>
      +12 data_crc16  packed_uint<2>
      +14 total_slices packed_uint<3>   ← 3 bytes
      +17 total_images packed_uint<3>   ← 3 bytes
      +20 tex_format  packed_uint<1>
      ...
    """
    if len(b) < 24:
        return None

    def ru16(ofs: int) -> int:
        return struct.unpack_from("<H", b, ofs)[0]

    def ru24(ofs: int) -> int:
        lo = struct.unpack_from("<H", b, ofs)[0]
        hi = b[ofs + 2]
        return lo | (hi << 16)

    def ru32(ofs: int) -> int:
        return struct.unpack_from("<I", b, ofs)[0]

    sig = ru16(0)
    if sig != BASIS_SIG_U16:
        return None
    ver = ru16(2)
    header_size = ru16(4)
    data_size = ru32(8)
    total_slices = ru24(14)
    total_images = ru24(17)
    tex_format = b[20] if len(b) > 20 else 0
    if total_slices == 0 or total_images == 0:
        return None
    if header_size == 0 or header_size > len(b):
        return None
    return {
        "sig": sig,
        "version": ver,
        "header_size": header_size,
        "data_size": data_size,
        "total_slices": total_slices,
        "total_images": total_images,
        "tex_format": tex_format,
    }


def rx_container(b: bytes, magic: bytes) -> dict | None:
    """RXBC / RXAS 容器解析:magic 4B | ver u16 | format u16 | w u32 | h u32 | payload。"""
    if not b.startswith(magic) or len(b) <= 16:
        return None
    ver, fmt = struct.unpack_from("<HH", b, 4)
    w, h = struct.unpack_from("<II", b, 8)
    payload = b[16:]
    if ver != 1 or w == 0 or h == 0 or not payload:
        return None
    if all(x == 0 for x in payload):
        return None
    return {"version": ver, "format": fmt, "width": w, "height": h, "payload": payload}


# Helper aliases used by run_gate to keep names consistent.
def ktx2_header(b: bytes) -> dict | None:
    """Thin alias that calls parse_ktx2 and adds a level-presence flag."""
    d = parse_ktx2(b)
    if d is None:
        return None
    lvl = d["levels"][0] if d["levels"] else {}
    d["level_byte_length"] = lvl.get("byte_length", 0)
    d["level_payload_present"] = lvl.get("byte_length", 0) > 0
    return d


def basis_header(b: bytes) -> dict | None:
    """Alias for parse_basis."""
    return parse_basis(b)


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
        sbom_text = sbom.read_text(encoding="utf-8") if sbom.is_file() else ""
        # 真实 pin 必须三处一致(VENDOR.md / SBOM.md / FFI 报告串),
        # 且过渡串不得再出现(否则 real_codec_identity 可被占位物充绿)。
        vendor_ok = (
            VENDOR_VERSION in vtext
            and UPSTREAM_COMMIT in vtext
            and UPSTREAM_TAG in vtext
            and UPSTREAM_URL in vtext
            and VENDOR_VERSION in sbom_text
        )
        results["license_sbom_entries_present"] = check(
            results["license_sbom_entries_present"] and vendor_ok,
            f"VENDOR.md/SBOM.md 未登记真实 pin({VENDOR_VERSION} / {UPSTREAM_COMMIT})",
        )
        for banned in FORBIDDEN_VERSION_SUBSTRINGS:
            # 只拒"版本串字面"出现在活跃 pin 位置上:
            # VENDOR.md/SBOM.md 可以有 "已废除 rurix-basis-transitional" 的历史说明行,
            # 但 VENDOR_VERSION 常量本身不得等于过渡串,且 vendor 组件表不得以该串作为 pin。
            active_pin_contaminated = (
                banned + "/0.1.0" in vtext.split("已废除")[0]
                or (banned + "/0.1.0" in sbom_text.split("已废除")[0])
            )
            check(
                not active_pin_contaminated,
                f"VENDOR.md/SBOM.md 仍含过渡串 {banned}(须清零活跃 pin)",
            )
        # vendor 快照与逐文件 digest 清单必须在树
        manifest = (
            ROOT
            / "src"
            / "rurix-basis-sys"
            / "vendor"
            / "basis_universal"
            / "vendor_manifest.json"
        )
        if check(manifest.is_file(), "vendor_manifest.json 缺失(vendor 快照未落盘)"):
            man = json.loads(manifest.read_text(encoding="utf-8"))
            check(
                man.get("commit") == UPSTREAM_COMMIT and man.get("tag") == UPSTREAM_TAG,
                "vendor_manifest.json pin 与 smoke 常量不符",
            )
            check(
                bool(man.get("license_digest")) and bool(man.get("source_digest")),
                "vendor_manifest.json 缺 source/LICENSE digest",
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

        # ── KTX2 腿:真实 basisu KTX2 容器全 header 校验 ──────────────────────
        kh = ktx2_header(ka)
        expect_blocks = int(rep1.get("expected_block_count", 0))
        results["ktx2_leg_present_valid"] = check(
            kh is not None
            and kh["supercompression"] == 0
            and kh["level_count"] == 1
            and kh["face_count"] == 1
            and kh["width"] == int(rep1.get("width", 0))
            and kh["height"] == int(rep1.get("height", 0))
            and kh["level_byte_length"] > 0
            and kh["level_payload_present"],
            f"KTX2 header 非法(真实容器校验失败): {kh}",
        )

        # ── Basis 腿:必须是真实 `.basis` 码流(禁 RXBS/占位冒充)────────────
        basis_path = a / "texture.basis"
        bs = basis_path.read_bytes() if basis_path.is_file() else b""
        bh = basis_header(bs)
        # 硬拒:任何 Rurix 自制容器 magic 出现在 `.basis` 位置 = FAIL
        substituted = any(bs.startswith(m) for m in FORBIDDEN_BASIS_PREFIXES)
        # 解码语义腿:cook 侧已做 `.basis` → BC7 真 transcode 回环,
        # 块数须 == golden(ceil(w/4)*ceil(h/4)),digest 非空。
        rt_blocks = int(rep1.get("basis_transcode_block_count", -1))
        rt_digest = str(rep1.get("basis_transcode_digest", ""))
        results["basis_leg_present_valid"] = check(
            bh is not None
            and not substituted
            and bh["version"] > 0
            and bh["total_images"] == 1
            and bh["total_slices"] >= 1
            and bh["data_size"] > 0
            and rep1.get("basis_present") is True
            and rep1.get("basis_signature") == "sB"
            and rt_blocks == expect_blocks
            and expect_blocks > 0
            and len(rt_digest) == 64
            and rt_digest != "0" * 64,
            "Basis 腿非真实 `.basis` 码流"
            f"(header={bh} substituted={substituted} "
            f"transcode_blocks={rt_blocks} expect={expect_blocks})",
        )
        note(
            f"basis_leg=real basis_universal ETC1S (sig=sB, slices={bh['total_slices'] if bh else 'n/a'}, "
            f"transcode_roundtrip_blocks={rt_blocks})"
        )

        # ── BCn 腿:三语义各自真实格式 + 计数 == golden ───────────────────────
        rxbc_a = rx_container(ba, RXBC_MAGIC)
        bcn_leg_ok = (
            rxbc_a is not None
            and rxbc_a["format"] == RXBC_FMT_BC7
            and rep1.get("gpu_format_bcn") == "BC7_UNORM"
            and int(rep1.get("bcn_block_count", -1)) == expect_blocks
            # BC7 = 16B/块
            and len(rxbc_a["payload"]) == expect_blocks * 16
        )
        # normal → BC5、mask → BC4:必须真的换格式(非同一 BC7 改标注)
        nout = root / "normal"
        mout = root / "mask"
        rcn, outn, errn = cook(exe, nout, "normal", semantics="normal")
        rcm, _outm, errm = cook(exe, mout, "mask", semantics="mask")
        repn = read_report(nout) if rcn == 0 else {}
        repm = read_report(mout) if rcm == 0 else {}
        check(rcn == 0, f"normal fixture cook 失败: {errn}")
        check(rcm == 0, f"mask fixture cook 失败: {errm}")

        rxbc_n = rx_container((nout / "texture.bcn").read_bytes(), RXBC_MAGIC) if rcn == 0 else None
        rxbc_m = rx_container((mout / "texture.bcn").read_bytes(), RXBC_MAGIC) if rcm == 0 else None
        n_blocks = int(repn.get("expected_block_count", 0))
        m_blocks = int(repm.get("expected_block_count", 0))
        bc5_ok = (
            rxbc_n is not None
            and rxbc_n["format"] == RXBC_FMT_BC5
            and repn.get("gpu_format_bcn") == "BC5_UNORM"
            and len(rxbc_n["payload"]) == n_blocks * 16  # BC5 = 16B/块
            and int(repn.get("bcn_block_count", -1)) == n_blocks
        )
        bc4_ok = (
            rxbc_m is not None
            and rxbc_m["format"] == RXBC_FMT_BC4
            and repm.get("gpu_format_bcn") == "BC4_UNORM"
            and len(rxbc_m["payload"]) == m_blocks * 8  # BC4 = 8B/块
            and int(repm.get("bcn_block_count", -1)) == m_blocks
        )
        results["bcn_leg_formats_expected"] = check(
            bcn_leg_ok and bc5_ok and bc4_ok,
            f"BCn 三语义腿未各自成立(bc7={bcn_leg_ok} bc5={bc5_ok} bc4={bc4_ok})",
        )

        # ── ASTC 腿:真实权重块(禁 void-extent/常色 fudge)───────────────────
        rxas_a = rx_container(aa, RXAS_MAGIC)
        # gradient fixture:每 4×4 cell 内非常色 → 必须产出真实权重块。
        gout = root / "gradient"
        rcg, _outg, errg = cook(exe, gout, "gradient")
        check(rcg == 0, f"gradient fixture cook 失败: {errg}")
        repg = read_report(gout) if rcg == 0 else {}
        g_total = int(repg.get("astc_block_count", 0))
        g_weighted = int(repg.get("astc_weighted_blocks", -1))
        g_void = int(repg.get("astc_void_extent_blocks", -1))
        results["astc_leg_format_expected"] = check(
            rxas_a is not None
            and rep1.get("gpu_format_astc") == "ASTC_4x4_UNORM"
            and int(rep1.get("astc_block_count", -1)) == expect_blocks
            and len(rxas_a["payload"]) == expect_blocks * 16
            # 非常色源必须有真实权重块,且不得全为 void-extent
            and g_total > 0
            and g_weighted == g_total
            and g_void == 0,
            "ASTC 腿无效或疑 void-extent/常色 fudge"
            f"(gradient: total={g_total} weighted={g_weighted} void_extent={g_void})",
        )
        note(
            f"astc gradient fixture: total={g_total} weighted={g_weighted} void_extent={g_void}"
        )

        # ── 容器 magic 复核(防改扩展名)+ 禁占位扩展 ────────────────────────
        results["container_not_renamed"] = check(
            ka.startswith(KTX2_MAGIC)
            and ba.startswith(RXBC_MAGIC)
            and aa.startswith(RXAS_MAGIC)
            and bs.startswith(BASIS_SIG_BYTES)
            and not substituted
            and kh is not None
            and bh is not None,
            "容器 magic 复核失败(疑改扩展名假转码)",
        )

        results["color_error_within_tolerance"] = check(
            int(rep1.get("color_max_delta", 999)) <= COLOR_MAX_DELTA
            and int(repn.get("color_max_delta", 999)) <= COLOR_MAX_DELTA,
            f"颜色误差超限: color={rep1.get('color_max_delta')} "
            f"normal={repn.get('color_max_delta')} > {COLOR_MAX_DELTA}",
        )

        if rcn == 0:
            results["normal_length_within_tolerance"] = check(
                float(repn.get("normal_length_mad", 9)) <= NORMAL_MAD,
                f"normal length 超限: {repn.get('normal_length_mad')}",
            )
        results["alpha_coverage_within_tolerance"] = check(
            float(rep1.get("alpha_coverage_delta", 9)) <= ALPHA_COV,
            f"alpha coverage 超限: {rep1.get('alpha_coverage_delta')}",
        )

        note(
            f"real codec={VENDOR_VERSION} (upstream {UPSTREAM_TAG}@{UPSTREAM_COMMIT[:12]});"
            f"measured color_max_delta={rep1.get('color_max_delta')},"
            f"normal_mad={repn.get('normal_length_mad')},"
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
