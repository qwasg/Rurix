#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-013: standalone real Windows D3D12 dispatch smoke for the particles_copy pass.

Template copy of ci/grx012_taa_resolve_d3d12_dispatch_smoke.py pointed at the
GRX-013 particles_copy package, adapted to STRUCTURED BUFFERS instead of textures:
it binds one StructuredBuffer<ParticleData> SRV (t0) and one
RWStructuredBuffer<float4> UAV (u0) with a 128-byte (32-dword) CopyPushConstant b0
root-constant block. It proves the *offline* particles_copy artifacts (the
DXC-compiled DXIL container, the Rurix-owned RTS0 root signature, and the
descriptor layout) can complete **one minimal compute dispatch on a real D3D12
device and command queue**, and additionally verifies every measured GPU output
instance against the CPU reference (the COPY_MODE_FILL_INSTANCES 3D subset from
the tracked ``generate_math_parity_evidence.py`` ``transform_instance`` reference)
within a small tolerance. It produces measured smoke evidence only. It does NOT:

  * mark the Godot runtime particles_copy pass as complete,
  * make the bridge default to RXGD_STATUS_OK,
  * claim any FPS / visual diff / measured fallback telemetry.

Discipline (mirrors the GRX-012 dispatch smoke):

  * The device/command queue are always real: fake/null handles are never
    accepted. If there is no hardware D3D12 adapter or no D3D12 runtime, the
    harness records ``status=skip`` with a concrete reason. SKIP never advances
    the ready gate.
  * The tracked DXIL / RTS0 / descriptor layout artifacts are used as-is. Their
    SHA-256 digests must match the tracked offline compile evidence, and the
    descriptor layout must match the particles_copy resource mapping
    (src_particles = structured_buffer t0, dst_instances = rwstructured_buffer u0,
    a 128-byte b0 root-constant block). Any mismatch is ``status=fail``.
  * The SRV/UAV/root-constant bindings are created strictly from the descriptor
    layout; the harness never guesses resource shapes.
  * The deterministic synthetic ParticleData AND the 128-byte b0 for each tracked
    math-parity case are generated in Python (the same f32 formulas the pass
    ships) and uploaded to the harness verbatim, so the ONLY GPU-vs-CPU divergence
    is the kernel math itself (sin/cos/rsqrt in the billboard cases).
  * A ``status=success`` run records adapter/device info, artifact hashes,
    dispatch dimensions, fence completion, and the measured-vs-CPU-reference
    comparison (every output instance within tolerance). It records
    ``real_d3d12_dispatch_recorded=true`` and ``cpu_reference_match=true`` (the
    two fields the GRX-013 gate reads); even so it keeps
    ``runtime_state=fallback_only`` and ``real_gpu_pass=false``.

If RURIX_REQUIRE_REAL=1, an environment that would otherwise SKIP becomes a hard
failure (exit 1); otherwise SKIP exits 0, matching the repo GPU-smoke policy.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import importlib.util
import json
import math
import os
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "particles_copy"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "particles_copy.dxil"
RTS0 = ARTIFACTS / "particles_copy.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "particles_copy_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_SCRIPT = PASS_DIR / "generate_math_parity_evidence.py"
EVIDENCE_OUT = PASS_DIR / "real_d3d12_dispatch_smoke.json"
WORK = ROOT / "target" / "grx013_d3d12_dispatch_smoke"

SUBJECT = "grx013_particles_copy_real_d3d12_dispatch_smoke"

# ParticleData stride (source) and 3D instance stride (destination, 5 vec4).
PARTICLE_STRIDE = 112
INSTANCE_STRIDE_VEC4 = 5
INSTANCE_STRIDE_BYTES = INSTANCE_STRIDE_VEC4 * 16

# The billboard cases carry sin/cos and normalize (rsqrt), whose GPU
# approximations differ from libm; the recorded max_abs_diff shows the real gap.
VALUE_TOLERANCE = 3.0e-4


def run(cmd: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True)


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def now_iso() -> str:
    return _dt.datetime.now().astimezone().replace(microsecond=0).isoformat()


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")


def locate_signed_dxc_dir() -> Path | None:
    dirs: list[Path] = []
    for key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        v = os.environ.get(key)
        if v:
            dirs.append(Path(v))
    dirs.append(KNOWN_DXC_DIR)
    for d in dirs:
        if (d / "dxil.dll").is_file():
            return d
    return None


def locate_dxcapi_include(dxc_dir: Path | None) -> Path | None:
    if dxc_dir is None:
        return None
    for base in (dxc_dir, *dxc_dir.parents):
        for name in ("inc", "include"):
            candidate = base / name / "dxcapi.h"
            if candidate.is_file():
                return candidate.parent
    return None


def locate_vcvars64() -> Path | None:
    override = os.environ.get("RURIX_VCVARS64")
    if override:
        p = Path(override)
        if p.is_file():
            return p
    candidates = [
        Path(r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"),
        Path(r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"),
    ]
    candidates.extend(Path(r"C:\Program Files").glob(r"Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat"))
    candidates.extend(Path(r"C:\Program Files (x86)").glob(r"Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat"))
    for p in candidates:
        if p.is_file():
            return p
    return None


def load_json(path: Path) -> dict | None:
    if not path.is_file():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def load_math_parity_reference():
    """Import the tracked particles_copy math-parity reference implementation so
    the Python check uses the SAME reference the pass ships."""
    spec = importlib.util.spec_from_file_location(
        "grx013_particles_copy_math_parity", MATH_PARITY_SCRIPT
    )
    if spec is None or spec.loader is None:
        return None
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except Exception:  # noqa: BLE001 - honest import failure, reported as skip
        return None
    return module


def offline_artifact_digests(evidence: dict) -> dict[str, str | None]:
    artifacts = evidence.get("artifacts")
    out: dict[str, str | None] = {"dxil": None, "root_signature": None, "descriptor_layout": None}
    if isinstance(artifacts, dict):
        for key in out:
            entry = artifacts.get(key)
            if isinstance(entry, dict):
                sha = entry.get("sha256")
                if isinstance(sha, str):
                    out[key] = sha
    return out


def descriptor_layout_matches_resource_mapping(layout: dict) -> str | None:
    """Return None when the descriptor layout matches the tracked GRX-013
    particles_copy resource mapping, otherwise a human-readable mismatch reason."""
    resources = layout.get("resources")
    expected = [
        ("src_particles", "t", 0, "structured_buffer"),
        ("dst_instances", "u", 0, "rwstructured_buffer"),
    ]
    if not isinstance(resources, list) or len(resources) != 2:
        return "descriptor layout does not declare exactly 2 resources"
    for i, (name, cls, reg, kind) in enumerate(expected):
        r = resources[i]
        if not (isinstance(r, dict) and r.get("name") == name and r.get("class") == cls
                and r.get("register") == reg and r.get("binding_kind") == kind):
            return f"resource[{i}] is not {name} {cls}{reg} (binding_kind {kind})"
    if layout.get("root_signature_parameters") != 2:
        return "root_signature_parameters != 2"
    if layout.get("root_constants") != 32:
        return "root_constants != 32"
    mapping = layout.get("grx013_mapping")
    if not isinstance(mapping, dict):
        return "missing grx013_mapping"
    if mapping.get("root_constant_bytes") != 128 or mapping.get("root_constant_dwords") != 32:
        return "root constant block is not 128 bytes / 32 dwords"
    if mapping.get("requires_64bit_integer_shader_capability") is not False:
        return "grx013_mapping must record requires_64bit_integer_shader_capability=false"
    if mapping.get("particle_data_stride_bytes") != PARTICLE_STRIDE:
        return "grx013_mapping particle_data_stride_bytes is not 112"
    names = [e.get("name") for e in layout.get("root_constant_layout", []) if isinstance(e, dict)]
    if names[:4] != ["sort_direction_x", "sort_direction_y", "sort_direction_z", "total_particles"]:
        return "root_constant_layout head does not match the CopyPushConstant contract"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx013-d3d12-dispatch-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx013-d3d12-dispatch-smoke] SKIP {msg}(降级 SKIP,退出 0)")
    write_evidence("skip", reason=msg, extra=extra or {})
    return 0


_EVIDENCE_BASE: dict = {}


def write_evidence(status: str, *, reason: str | None = None, extra: dict | None = None) -> None:
    doc = dict(_EVIDENCE_BASE)
    doc["status"] = status
    doc["timestamp"] = now_iso()
    doc["run_url"] = github_run_url()
    if reason is not None:
        doc["reason"] = reason
    if extra:
        doc.update(extra)
    EVIDENCE_OUT.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE_OUT.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx013-d3d12-dispatch-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


# ---------------------------------------------------------------------------
# Params-file builders (Python owns the exact bytes; the harness uploads them).
#
# Params binary format (little-endian):
#   uint32 particle_count
#   uint32 src_bytes            (particle_count * 112)
#   uint32 dst_bytes            (particle_count * 5 * 16)
#   uint8  b0[128]              (the 32-dword CopyPushConstant mirror)
#   uint8  src_particles[src_bytes]
# ---------------------------------------------------------------------------
def build_particle_bytes(parity, n: int) -> bytes:
    """Pack n ParticleData structs (112 bytes each) with the SAME f32 values the
    tracked reference generates, matching the HLSL struct layout."""
    out = bytearray()
    for p in range(n):
        c0, c1, c2, c3 = parity.xform_columns(p)
        vel = parity.velocity(p)
        color = parity.color(p)
        custom = parity.custom(p)
        out += struct.pack("<4f", *c0)
        out += struct.pack("<4f", *c1)
        out += struct.pack("<4f", *c2)
        out += struct.pack("<4f", *c3)
        out += struct.pack("<3f", *vel)
        out += struct.pack("<I", parity.flags(p))
        out += struct.pack("<4f", *color)
        out += struct.pack("<4f", *custom)
    assert len(out) == n * PARTICLE_STRIDE
    return bytes(out)


def build_b0(parity, consts: dict) -> bytes:
    """The 32-dword (128-byte) CopyPushConstant mirror for a case (descriptor
    layout dword order). Out-of-scope fields are neutral (0)."""
    f32 = parity.f32
    sort = [f32(x) for x in consts["sort_direction"]]
    up = [f32(x) for x in consts["align_up"]]
    b = bytearray(128)
    struct.pack_into("<3f", b, 0, *sort)                              # dwords 0-2
    struct.pack_into("<I", b, 12, int(consts["total_particles"]))    # dword 3
    struct.pack_into("<I", b, 16, 1)                                 # dword 4 trail_size
    struct.pack_into("<I", b, 20, 1)                                 # dword 5 trail_total
    struct.pack_into("<f", b, 24, 0.0)                               # dword 6 frame_delta
    struct.pack_into("<f", b, 28, f32(consts["frame_remainder"]))   # dword 7
    struct.pack_into("<3f", b, 32, *up)                              # dwords 8-10
    struct.pack_into("<I", b, 44, int(consts["align_mode"]))        # dword 11
    # dwords 12-27 (lifetime_split/reverse, motion_vectors_offset, flags_bits,
    # inv_emission_transform[12]) stay 0.
    struct.pack_into("<I", b, 112, int(consts["align_channel_filter"]))  # dword 28
    # dwords 29-31 (align_axis, pad1, pad2) stay 0.
    return bytes(b)


def parity_cases(parity) -> list[dict]:
    """Rebuild the tracked case constants (mirrors generate_math_parity_evidence
    main()); each case is (case_id, n, consts)."""
    sort_a = [0.6, 0.0, 0.8]
    sort_b = [0.48, 0.64, 0.6]
    up_y = [0.0, 1.0, 0.0]
    return [
        {"case_id": "fill_instances_align_disabled", "n": 8, "consts": {
            "align_mode": parity.ALIGN_DISABLED, "align_channel_filter": 0,
            "sort_direction": sort_a, "align_up": up_y, "frame_remainder": 0.5}},
        {"case_id": "fill_instances_billboard_channel_x", "n": 8, "consts": {
            "align_mode": parity.ALIGN_BILLBOARD, "align_channel_filter": 1,
            "sort_direction": sort_a, "align_up": up_y, "frame_remainder": 0.0}},
        {"case_id": "fill_instances_billboard_channel_w", "n": 6, "consts": {
            "align_mode": parity.ALIGN_BILLBOARD, "align_channel_filter": 4,
            "sort_direction": sort_b, "align_up": up_y, "frame_remainder": 0.25}},
        {"case_id": "fill_instances_billboard_channel_none", "n": 5, "consts": {
            "align_mode": parity.ALIGN_BILLBOARD, "align_channel_filter": 0,
            "sort_direction": sort_a, "align_up": up_y, "frame_remainder": 0.0}},
    ]


def compare_instances(parity, n: int, consts: dict, out_bin: Path) -> dict:
    """Compare every GPU-observed instance (5 vec4) against the tracked
    transform_instance reference."""
    consts_full = dict(consts)
    consts_full["total_particles"] = n
    raw = out_bin.read_bytes()
    expected_len = n * INSTANCE_STRIDE_BYTES
    if len(raw) != expected_len:
        return {"match": False, "reason": f"output binary size {len(raw)} != {expected_len}"}
    obs = struct.unpack(f"<{n * INSTANCE_STRIDE_VEC4 * 4}f", raw)
    max_abs = 0.0
    mismatched = 0
    worst = None
    for p in range(n):
        expected = parity.transform_instance(p, consts_full)  # 5 vec4
        for k in range(INSTANCE_STRIDE_VEC4):
            for c in range(4):
                r = expected[k][c]
                o = obs[(p * INSTANCE_STRIDE_VEC4 + k) * 4 + c]
                if math.isinf(r) or math.isnan(r):
                    ok = (math.isinf(o) and (o > 0) == (r > 0)) or (math.isnan(r) and math.isnan(o))
                    if not ok:
                        mismatched += 1
                        if worst is None:
                            worst = {"instance": p, "vec4": k, "comp": c, "observed": o, "reference": r}
                    continue
                d = abs(o - r)
                if d > max_abs:
                    max_abs = d
                    worst = {"instance": p, "vec4": k, "comp": c, "observed": o, "reference": r}
                if d > VALUE_TOLERANCE:
                    mismatched += 1
    return {
        "match": mismatched == 0,
        "max_abs_diff": max_abs,
        "mismatched_components": mismatched,
        "total_instances": n,
        "value_tolerance": VALUE_TOLERANCE,
        "worst": worst,
    }


# ---------------------------------------------------------------------------
# Real D3D12 structured-buffer compute-dispatch harness (C++/MSVC), on demand.
#
# argv: <dxil> <rts0> <params_bin> <out_bin> [dxil.dll]
# Exit codes: 0 = success, 1 = fail, 2 = skip (no adapter / runtime).
#
# Root signature is created DIRECTLY from the Rurix RTS0 bytes, the compute PSO
# from the Rurix DXIL container, and the descriptor table is bound per the
# descriptor layout:
#   root param 0 = 32-dword (128-byte) b0 root constants
#   root param 1 = descriptor table [ SRV t0 (StructuredBuffer<ParticleData>),
#                  UAV u0 (RWStructuredBuffer<float4>) ]
#
# The src_particles bytes + the b0 come verbatim from the params file (Python
# built them with the tracked f32 formulas). The harness writes the full float32
# dst_instances readback (tight N*5*4 row-major) to <out_bin>; the Python side
# re-verifies every instance against the tracked transform_instance reference.
# ---------------------------------------------------------------------------
HARNESS_CPP = r"""#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>

#include <dxcapi.h>

using Microsoft::WRL::ComPtr;

static const UINT PARTICLE_STRIDE = 112u;
static const UINT INSTANCE_STRIDE = 80u; // 5 * float4

static int fail_hr(const char* what, HRESULT hr) {
    std::fprintf(stderr, "RXGD_DISPATCH: fail %s hr=0x%08lx\n", what, (unsigned long)hr);
    return 1;
}
static int fail_msg(const char* what) {
    std::fprintf(stderr, "RXGD_DISPATCH: fail %s\n", what);
    return 1;
}
static int skip_msg(const char* what) {
    std::fprintf(stderr, "RXGD_DISPATCH: skip %s\n", what);
    return 2;
}

static std::vector<uint8_t> read_file(const wchar_t* path, bool* ok) {
    *ok = false;
    std::ifstream f(path, std::ios::binary);
    if (!f) return {};
    f.seekg(0, std::ios::end);
    const auto n = f.tellg();
    if (n <= 0) return {};
    f.seekg(0, std::ios::beg);
    std::vector<uint8_t> data((size_t)n);
    f.read(reinterpret_cast<char*>(data.data()), n);
    if (!f) return {};
    *ok = true;
    return data;
}

static D3D12_HEAP_PROPERTIES heap_props(D3D12_HEAP_TYPE type) {
    D3D12_HEAP_PROPERTIES hp = {};
    hp.Type = type;
    hp.CreationNodeMask = 1;
    hp.VisibleNodeMask = 1;
    return hp;
}
static D3D12_RESOURCE_DESC buffer_desc(UINT64 bytes, D3D12_RESOURCE_FLAGS flags) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    d.Width = bytes;
    d.Height = 1;
    d.DepthOrArraySize = 1;
    d.MipLevels = 1;
    d.Format = DXGI_FORMAT_UNKNOWN;
    d.SampleDesc.Count = 1;
    d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    d.Flags = flags;
    return d;
}
static std::string narrow(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string out((size_t)std::max(n - 1, 0), '\0');
    if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr);
    return out;
}

struct MemBlob : public IDxcBlob {
    LONG m_ref; void* m_ptr; SIZE_T m_size;
    MemBlob(void* p, SIZE_T s) : m_ref(1), m_ptr(p), m_size(s) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == __uuidof(IUnknown) || riid == __uuidof(IDxcBlob)) {
            *ppv = static_cast<IDxcBlob*>(this); AddRef(); return S_OK;
        }
        *ppv = nullptr; return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return (ULONG)InterlockedIncrement(&m_ref); }
    ULONG STDMETHODCALLTYPE Release() override { return (ULONG)InterlockedDecrement(&m_ref); }
    LPVOID STDMETHODCALLTYPE GetBufferPointer() override { return m_ptr; }
    SIZE_T STDMETHODCALLTYPE GetBufferSize() override { return m_size; }
};

static bool sign_dxil_in_place(std::vector<uint8_t>& dxil, const wchar_t* dxil_dll, std::string* err) {
    HMODULE lib = dxil_dll ? LoadLibraryW(dxil_dll) : LoadLibraryW(L"dxil.dll");
    if (!lib) { *err = "LoadLibrary(dxil.dll) failed"; return false; }
    auto create = reinterpret_cast<DxcCreateInstanceProc>(GetProcAddress(lib, "DxcCreateInstance"));
    if (!create) { *err = "GetProcAddress(DxcCreateInstance) failed"; return false; }
    IDxcValidator* validator = nullptr;
    if (FAILED(create(CLSID_DxcValidator, __uuidof(IDxcValidator),
                      reinterpret_cast<void**>(&validator))) || !validator) {
        *err = "DxcCreateInstance(CLSID_DxcValidator) failed"; return false;
    }
    MemBlob blob(dxil.data(), dxil.size());
    IDxcOperationResult* result = nullptr;
    HRESULT hr = validator->Validate(&blob, DxcValidatorFlags_InPlaceEdit, &result);
    bool ok = false;
    if (SUCCEEDED(hr) && result) {
        HRESULT status = E_FAIL; result->GetStatus(&status);
        ok = SUCCEEDED(status);
        if (!ok) *err = "validator rejected the DXIL container";
    } else { *err = "IDxcValidator::Validate failed"; }
    if (result) result->Release();
    validator->Release();
    return ok;
}

// Create a DEFAULT-heap buffer, optionally fill it from an upload buffer, and
// record a copy + transition to `after` on `cmd`.
static bool make_buffer(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,
                        UINT64 bytes, D3D12_RESOURCE_FLAGS flags,
                        D3D12_RESOURCE_STATES after, const uint8_t* data,
                        ComPtr<ID3D12Resource>& buf, ComPtr<ID3D12Resource>& upload,
                        const char* label) {
    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    auto desc = buffer_desc(bytes, flags);
    D3D12_RESOURCE_STATES initial = data ? D3D12_RESOURCE_STATE_COPY_DEST : after;
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &desc,
                                               initial, nullptr, IID_PPV_ARGS(&buf)))) {
        std::fprintf(stderr, "RXGD_DISPATCH: fail CreateCommittedResource(%s)\n", label);
        return false;
    }
    if (data) {
        auto upload_heap = heap_props(D3D12_HEAP_TYPE_UPLOAD);
        auto up_desc = buffer_desc(bytes, D3D12_RESOURCE_FLAG_NONE);
        if (FAILED(device->CreateCommittedResource(&upload_heap, D3D12_HEAP_FLAG_NONE, &up_desc,
                                                   D3D12_RESOURCE_STATE_GENERIC_READ, nullptr,
                                                   IID_PPV_ARGS(&upload)))) {
            std::fprintf(stderr, "RXGD_DISPATCH: fail CreateCommittedResource(%s upload)\n", label);
            return false;
        }
        uint8_t* p = nullptr;
        D3D12_RANGE empty = {0, 0};
        if (FAILED(upload->Map(0, &empty, reinterpret_cast<void**>(&p)))) {
            std::fprintf(stderr, "RXGD_DISPATCH: fail Map(%s upload)\n", label);
            return false;
        }
        std::memcpy(p, data, (size_t)bytes);
        upload->Unmap(0, nullptr);
        cmd->CopyBufferRegion(buf.Get(), 0, upload.Get(), 0, bytes);
        D3D12_RESOURCE_BARRIER b = {};
        b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        b.Transition.pResource = buf.Get();
        b.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST;
        b.Transition.StateAfter = after;
        b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &b);
    }
    return true;
}

int wmain(int argc, wchar_t** argv) {
    if (argc < 5 || argc > 6) return fail_msg("usage: harness dxil rts0 params_bin out_bin [dxil.dll]");
    bool ok_dxil = false, ok_rts0 = false, ok_params = false;
    auto dxil = read_file(argv[1], &ok_dxil);
    const auto rts0 = read_file(argv[2], &ok_rts0);
    const auto params = read_file(argv[3], &ok_params);
    if (!ok_dxil || dxil.empty()) return fail_msg("read dxil");
    if (!ok_rts0 || rts0.empty()) return fail_msg("read rts0");
    if (!ok_params || params.size() < 12 + 128) return fail_msg("read params");
    const wchar_t* out_bin = argv[4];
    const wchar_t* dxil_dll = (argc >= 6) ? argv[5] : nullptr;

    UINT particle_count = 0, src_bytes = 0, dst_bytes = 0;
    std::memcpy(&particle_count, params.data() + 0, 4);
    std::memcpy(&src_bytes, params.data() + 4, 4);
    std::memcpy(&dst_bytes, params.data() + 8, 4);
    const uint8_t* b0 = params.data() + 12;
    const uint8_t* src_data = params.data() + 12 + 128;
    if (params.size() != (size_t)(12 + 128 + src_bytes)) return fail_msg("params size mismatch");
    if (src_bytes != particle_count * PARTICLE_STRIDE) return fail_msg("src_bytes mismatch");
    if (dst_bytes != particle_count * INSTANCE_STRIDE) return fail_msg("dst_bytes mismatch");

    bool experimental = false;
    {
        static const GUID kExp = D3D12ExperimentalShaderModels;
        experimental = SUCCEEDED(D3D12EnableExperimentalFeatures(1, &kExp, nullptr, nullptr));
    }
    std::printf("RXGD_DISPATCH: experimental_shader_models=%s\n", experimental ? "on" : "off");

    std::string sign_err;
    const bool dxil_signed = sign_dxil_in_place(dxil, dxil_dll, &sign_err);
    std::printf("RXGD_DISPATCH: dxil_signed_for_load=%s\n", dxil_signed ? "yes" : "no");
    if (!dxil_signed) std::fprintf(stderr, "RXGD_DISPATCH: sign note: %s\n", sign_err.c_str());

    ComPtr<IDXGIFactory6> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory))))
        return skip_msg("no DXGI factory (no D3D12 runtime)");

    ComPtr<IDXGIAdapter1> chosen;
    DXGI_ADAPTER_DESC1 chosen_desc = {};
    SIZE_T best_mem = 0;
    for (UINT i = 0;; ++i) {
        ComPtr<IDXGIAdapter1> adapter;
        HRESULT e = factory->EnumAdapters1(i, &adapter);
        if (e == DXGI_ERROR_NOT_FOUND) break;
        if (FAILED(e)) break;
        DXGI_ADAPTER_DESC1 d = {};
        adapter->GetDesc1(&d);
        if (d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) continue;
        if (SUCCEEDED(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0,
                                        __uuidof(ID3D12Device), nullptr)) &&
            d.DedicatedVideoMemory >= best_mem) {
            best_mem = d.DedicatedVideoMemory;
            chosen = adapter;
            chosen_desc = d;
        }
    }
    if (!chosen) return skip_msg("no hardware D3D12 adapter");

    ComPtr<ID3D12Device> device;
    if (FAILED(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device))))
        return skip_msg("D3D12CreateDevice failed on hardware adapter");
    std::printf("RXGD_DISPATCH: adapter=\"%s\"\n", narrow(chosen_desc.Description).c_str());

    ComPtr<ID3D12RootSignature> root;
    HRESULT hr_root = device->CreateRootSignature(0, rts0.data(), rts0.size(), IID_PPV_ARGS(&root));
    if (FAILED(hr_root)) return fail_hr("CreateRootSignature(rurix rts0)", hr_root);

    D3D12_COMPUTE_PIPELINE_STATE_DESC pd = {};
    pd.pRootSignature = root.Get();
    pd.CS = {dxil.data(), dxil.size()};
    ComPtr<ID3D12PipelineState> pso;
    HRESULT hr_pso = device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&pso));
    if (FAILED(hr_pso)) return fail_hr("CreateComputePipelineState(rurix dxil)", hr_pso);

    D3D12_COMMAND_QUEUE_DESC qd = {};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue;
    if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)))) return fail_msg("CreateCommandQueue");
    ComPtr<ID3D12CommandAllocator> alloc;
    if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc))))
        return fail_msg("CreateCommandAllocator");
    ComPtr<ID3D12GraphicsCommandList> cmd;
    if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(),
                                        pso.Get(), IID_PPV_ARGS(&cmd))))
        return fail_msg("CreateCommandList");

    // src_particles (SRV, uploaded) + dst_instances (UAV).
    ComPtr<ID3D12Resource> src_buf, src_upload, dst_buf, dst_upload;
    if (!make_buffer(device.Get(), cmd.Get(), src_bytes, D3D12_RESOURCE_FLAG_NONE,
                     D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, src_data, src_buf, src_upload, "src"))
        return 1;
    if (!make_buffer(device.Get(), cmd.Get(), dst_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
                     D3D12_RESOURCE_STATE_UNORDERED_ACCESS, nullptr, dst_buf, dst_upload, "dst"))
        return 1;

    // Descriptor heap: [SRV t0, UAV u0].
    D3D12_DESCRIPTOR_HEAP_DESC hd = {};
    hd.NumDescriptors = 2;
    hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap;
    if (FAILED(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap))))
        return fail_msg("CreateDescriptorHeap(cbv_srv_uav)");
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    D3D12_CPU_DESCRIPTOR_HANDLE cpu = heap->GetCPUDescriptorHandleForHeapStart();
    {
        D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
        srv.Format = DXGI_FORMAT_UNKNOWN;
        srv.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
        srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
        srv.Buffer.FirstElement = 0;
        srv.Buffer.NumElements = particle_count;
        srv.Buffer.StructureByteStride = PARTICLE_STRIDE;
        srv.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
        device->CreateShaderResourceView(src_buf.Get(), &srv, cpu);
    }
    {
        D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
        uav.Format = DXGI_FORMAT_UNKNOWN;
        uav.ViewDimension = D3D12_UAV_DIMENSION_BUFFER;
        uav.Buffer.FirstElement = 0;
        uav.Buffer.NumElements = particle_count * 5u;
        uav.Buffer.StructureByteStride = 16u;
        uav.Buffer.CounterOffsetInBytes = 0;
        uav.Buffer.Flags = D3D12_BUFFER_UAV_FLAG_NONE;
        D3D12_CPU_DESCRIPTOR_HANDLE h = cpu; h.ptr += (SIZE_T)inc;
        device->CreateUnorderedAccessView(dst_buf.Get(), nullptr, &uav, h);
    }

    // Readback buffer for the dst UAV.
    auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);
    auto rb_desc = buffer_desc(dst_bytes, D3D12_RESOURCE_FLAG_NONE);
    ComPtr<ID3D12Resource> readback;
    if (FAILED(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                               D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                               IID_PPV_ARGS(&readback))))
        return fail_msg("CreateCommittedResource(readback)");

    // Bind + dispatch.
    cmd->SetComputeRootSignature(root.Get());
    ID3D12DescriptorHeap* heaps[] = {heap.Get()};
    cmd->SetDescriptorHeaps(1, heaps);
    uint32_t rc[32];
    std::memcpy(rc, b0, 128);
    cmd->SetComputeRoot32BitConstants(0, 32, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetPipelineState(pso.Get());
    const UINT gx = std::max<UINT>((particle_count + 63u) / 64u, 1u);
    cmd->Dispatch(gx, 1, 1);

    D3D12_RESOURCE_BARRIER db = {};
    db.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    db.Transition.pResource = dst_buf.Get();
    db.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    db.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
    db.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &db);
    cmd->CopyBufferRegion(readback.Get(), 0, dst_buf.Get(), 0, dst_bytes);
    if (FAILED(cmd->Close())) return fail_msg("Close command list");

    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence;
    if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) return fail_msg("CreateFence");
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!ev) return fail_msg("CreateEvent");
    if (FAILED(queue->Signal(fence.Get(), 1))) return fail_msg("Signal fence");
    if (fence->GetCompletedValue() < 1) {
        if (FAILED(fence->SetEventOnCompletion(1, ev))) return fail_msg("SetEventOnCompletion");
        WaitForSingleObject(ev, INFINITE);
    }
    CloseHandle(ev);
    const UINT64 fence_done = fence->GetCompletedValue();
    if (fence_done < 1) return fail_msg("fence did not reach completion");

    uint8_t* mapped = nullptr;
    D3D12_RANGE range = {0, (SIZE_T)dst_bytes};
    if (FAILED(readback->Map(0, &range, reinterpret_cast<void**>(&mapped)))) return fail_msg("Map readback");
    std::ofstream of(out_bin, std::ios::binary);
    if (!of) return fail_msg("open out_bin");
    of.write(reinterpret_cast<const char*>(mapped), (std::streamsize)dst_bytes);
    of.close();
    uint32_t checksum = 2166136261u;
    for (UINT i = 0; i < dst_bytes; ++i) { checksum ^= mapped[i]; checksum *= 16777619u; }
    readback->Unmap(0, nullptr);
    if (!of) return fail_msg("write out_bin");

    std::printf("RXGD_DISPATCH: ok adapter=\"%s\" dispatch=%u,1,1 fence=%llu instances=%u checksum=0x%08x\n",
                narrow(chosen_desc.Description).c_str(), gx,
                (unsigned long long)fence_done, particle_count, checksum);
    return 0;
}
"""


def compile_harness(vcvars: Path, cpp: Path, exe: Path, include_dir: Path | None) -> tuple[bool, str]:
    obj = WORK / "harness.obj"
    bat = WORK / "build_dispatch_smoke.bat"
    include_flag = f'/I "{include_dir}" ' if include_dir is not None else ""
    bat.write_text(
        "@echo off\n"
        f'call "{vcvars}" >nul\n'
        "if errorlevel 1 exit /b %errorlevel%\n"
        f'cl /nologo /std:c++17 /EHsc /W4 /O2 /fp:precise /DUNICODE /D_UNICODE {include_flag}"{cpp}" '
        f'/Fe:"{exe}" /Fo:"{obj}" /link d3d12.lib dxgi.lib\n',
        encoding="utf-8",
    )
    p = subprocess.run(["cmd.exe", "/d", "/c", str(bat)], cwd=WORK, capture_output=True, text=True)
    log = (p.stdout + p.stderr).strip()
    if p.returncode != 0 or not exe.is_file():
        return False, log[-3000:]
    return True, log[-2000:]


def parse_harness_output(output: str) -> dict:
    parsed: dict = {}
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("RXGD_DISPATCH: experimental_shader_models="):
            parsed["experimental_shader_models"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_DISPATCH: dxil_signed_for_load="):
            parsed["dxil_signed_for_load"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_DISPATCH: ok "):
            for token in ("dispatch=", "fence=", "instances=", "checksum="):
                idx = line.find(token)
                if idx >= 0:
                    parsed[token.rstrip("=")] = line[idx + len(token):].split(" ", 1)[0]
            a0 = line.find('adapter="')
            if a0 >= 0:
                a0 += len('adapter="')
                a1 = line.find('"', a0)
                if a1 > a0:
                    parsed["adapter"] = line[a0:a1]
    return parsed


def main() -> int:
    global _EVIDENCE_BASE

    for path in (DXIL, RTS0, DESCRIPTOR_LAYOUT, OFFLINE_EVIDENCE, MATH_PARITY_SCRIPT):
        if not path.is_file():
            _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
            return fail(f"required artifact missing: {path.relative_to(ROOT)}")

    dxil_sha = sha256_file(DXIL)
    rts0_sha = sha256_file(RTS0)
    layout_sha = sha256_file(DESCRIPTOR_LAYOUT)
    offline = load_json(OFFLINE_EVIDENCE)
    layout = load_json(DESCRIPTOR_LAYOUT)
    if offline is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read offline_compile_evidence.json")
    if layout is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read particles_copy_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "particles_copy",
        "segment": "standalone_dispatch_smoke",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "cpu_reference_match": False,
        "artifacts": {
            "dxil": {"path": str(DXIL.relative_to(ROOT)).replace("\\", "/"), "sha256": dxil_sha},
            "root_signature": {"path": str(RTS0.relative_to(ROOT)).replace("\\", "/"), "sha256": rts0_sha},
            "descriptor_layout": {
                "path": str(DESCRIPTOR_LAYOUT.relative_to(ROOT)).replace("\\", "/"),
                "sha256": layout_sha,
            },
        },
        "offline_evidence": {
            "path": str(OFFLINE_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
            "dxil_sha256": offline_digests["dxil"],
            "root_signature_sha256": offline_digests["root_signature"],
            "descriptor_layout_sha256": offline_digests["descriptor_layout"],
        },
        "artifact_hashes_match_offline_evidence": (
            dxil_sha == offline_digests["dxil"]
            and rts0_sha == offline_digests["root_signature"]
            and layout_sha == offline_digests["descriptor_layout"]
        ),
        "note": (
            "GRX-013 standalone real D3D12 structured-buffer dispatch smoke evidence "
            "only. A success flips real_d3d12_dispatch_recorded/cpu_reference_match "
            "true (the fields the GRX-013 gate reads) but keeps "
            "runtime_state=fallback_only and real_gpu_pass=false; it is not a Godot "
            "runtime pass, visual, perf, or measured-telemetry claim."
        ),
    }

    if not _EVIDENCE_BASE["artifact_hashes_match_offline_evidence"]:
        return fail(
            "artifact SHA-256 does not match tracked offline compile evidence "
            f"(dxil={dxil_sha} vs {offline_digests['dxil']}, "
            f"rts0={rts0_sha} vs {offline_digests['root_signature']}, "
            f"layout={layout_sha} vs {offline_digests['descriptor_layout']})"
        )

    layout_issue = descriptor_layout_matches_resource_mapping(layout)
    if layout_issue is not None:
        return fail(f"descriptor layout / resource mapping mismatch: {layout_issue}")

    parity = load_math_parity_reference()
    if parity is None or not hasattr(parity, "transform_instance"):
        return fail(
            "cannot import the tracked generate_math_parity_evidence.py reference "
            "implementation (transform_instance) for the CPU cross-check"
        )

    vcvars = locate_vcvars64()
    if vcvars is None:
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64);无法编译真实 D3D12 dispatch harness")

    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip(
            "未找到含 dxil.dll 的签名 DXC pin(set RURIX_DXC_DIR=H:\\dxc-round7\\extracted\\bin\\x64);"
            "无法为 DXC 产出的 DXIL container 在内存中签名以在非 Developer-Mode device 上加载"
        )
    include_dir = locate_dxcapi_include(dxc_dir)
    if include_dir is None:
        return skip(f"未在 {dxc_dir} 附近找到 dxcapi.h(签名路径无法编译)")
    dxil_dll = dxc_dir / "dxil.dll"

    WORK.mkdir(parents=True, exist_ok=True)
    cpp = WORK / "particles_copy_dispatch_harness.cpp"
    exe = WORK / "particles_copy_dispatch_harness.exe"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")

    built, build_log = compile_harness(vcvars, cpp, exe, include_dir)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 dispatch harness 失败(可能缺 Windows SDK D3D12 头/库)",
                    extra={"build_log_tail": build_log})

    cases = parity_cases(parity)
    device_info: dict = {}
    case_results: list[dict] = []
    all_match = True
    for case in cases:
        n = case["n"]
        consts = case["consts"]
        b0 = build_b0(parity, {**consts, "total_particles": n})
        src = build_particle_bytes(parity, n)
        params = struct.pack("<III", n, n * PARTICLE_STRIDE, n * INSTANCE_STRIDE_BYTES) + b0 + src
        params_bin = WORK / f"params_{case['case_id']}.bin"
        out_bin = WORK / f"out_{case['case_id']}.bin"
        params_bin.write_bytes(params)
        if out_bin.exists():
            out_bin.unlink()

        p = run([str(exe), str(DXIL), str(RTS0), str(params_bin), str(out_bin), str(dxil_dll)], cwd=WORK)
        output = (p.stdout + p.stderr).strip()
        print(f"--- case {case['case_id']} ---")
        print(output)
        parsed = parse_harness_output(output)
        if not device_info:
            device_info = {
                "adapter": parsed.get("adapter"),
                "experimental_shader_models": parsed.get("experimental_shader_models"),
                "dxil_signed_for_load": parsed.get("dxil_signed_for_load"),
                "dxil_validator": str(dxil_dll).replace("\\", "/"),
            }

        if p.returncode == 2:
            return skip("no real D3D12 device harness available (see harness output)",
                        extra={"device": device_info, "stdout": output})
        if p.returncode != 0 or "RXGD_DISPATCH: ok" not in output or not out_bin.is_file():
            return fail(f"real D3D12 particles_copy dispatch smoke failed for case {case['case_id']}",
                        extra={"device": device_info, "exit_code": p.returncode, "stdout": output})

        comparison = compare_instances(parity, n, consts, out_bin)
        if not comparison.get("match"):
            all_match = False
        case_results.append({
            "case_id": case["case_id"],
            "particle_count": n,
            "align_mode": int(consts["align_mode"]),
            "align_channel_filter": int(consts["align_channel_filter"]),
            "dispatch": parsed.get("dispatch"),
            "fence_completed_value": parsed.get("fence"),
            "readback_checksum": parsed.get("checksum"),
            "comparison": comparison,
        })

    if not all_match:
        return fail(
            "GPU-observed instances did not match the tracked transform_instance reference",
            extra={"device": device_info, "cases": case_results},
        )

    cpu_reference = {
        "reference_impl": (
            "spike/godot-rurix/passes/particles_copy/generate_math_parity_evidence.py "
            "transform_instance (imported; every instance compared over 5 vec4 in Python)"
        ),
        "value_tolerance": VALUE_TOLERANCE,
        "cases": case_results,
    }
    max_diff = max((c["comparison"].get("max_abs_diff", 0.0) for c in case_results), default=0.0)
    write_evidence(
        "success",
        extra={
            "real_d3d12_dispatch_recorded": True,
            "cpu_reference_match": True,
            "device": device_info,
            "cpu_reference": cpu_reference,
            "checks": {
                "artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "root_signature_create_from_rurix_rts0": True,
                "compute_pso_from_rurix_dxil": True,
                "two_structured_buffers_bound_from_layout": True,
                "dispatch_executed": True,
                "fence_completed": True,
                "output_uav_readback": True,
                "all_output_instances_match_cpu_reference": True,
            },
        },
    )
    print(f"[grx013-d3d12-dispatch-smoke] PASS measured real D3D12 dispatch over "
          f"{len(case_results)} cases; adapter={device_info.get('adapter')} max_abs_diff={max_diff}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
