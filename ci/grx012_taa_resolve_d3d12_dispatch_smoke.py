#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-012: standalone real Windows D3D12 dispatch smoke for the taa_resolve pass.

Template copy of ci/grx011_ssao_blur_d3d12_dispatch_smoke.py pointed at the
GRX-012 taa_resolve package, extended to bind 6 resources (5 SRVs t0..t4 + 1
UAV u0). It proves the *offline* taa_resolve artifacts (the DXC-compiled DXIL
container, the Rurix-owned RTS0 root signature, and the descriptor layout) can
complete **one minimal compute dispatch on a real D3D12 device and command
queue**, and additionally verifies every measured GPU output texel against the
CPU reference (the single-frame TAA resolve from the tracked
``generate_math_parity_evidence.py`` reference implementation) within a small
tolerance. It produces measured smoke evidence only. It does NOT:

  * mark the Godot runtime taa_resolve pass as complete,
  * make the bridge default to RXGD_STATUS_OK,
  * claim any FPS / visual diff / measured fallback telemetry.

Discipline (mirrors the GRX-011 dispatch smoke):

  * The device/command queue are always real: fake/null handles are never
    accepted. If there is no hardware D3D12 adapter or no D3D12 runtime, the
    harness records ``status=skip`` with a concrete reason. SKIP never advances
    the ready gate.
  * The tracked DXIL / RTS0 / descriptor layout artifacts are used as-is. Their
    SHA-256 digests must match the tracked offline compile evidence, and the
    descriptor layout must match the taa_resolve resource mapping (color=t0,
    depth=t1, velocity=t2, last_velocity=t3, history=t4 SRVs + output=u0 UAV, a
    28-byte b0 root-constant block). Any mismatch is ``status=fail``.
  * The SRV/UAV/root-constant bindings are created strictly from the descriptor
    layout; the harness never guesses resource shapes.
  * A ``status=success`` run records adapter/device info, artifact hashes,
    dispatch dimensions, fence completion, and the measured-vs-CPU-reference
    comparison (every output texel within tolerance). It records
    ``real_d3d12_dispatch_recorded=true`` and ``cpu_reference_match=true`` (the
    two fields the GRX-012 gate reads); even so it keeps
    ``runtime_state=fallback_only`` and ``real_gpu_pass=false``.

The single reference implementation is the tracked
``generate_math_parity_evidence.py`` ``taa_resolve_frame``: the C++ harness
dispatches and writes the full float32 RGBA readback to a binary file, and the
Python side recomputes the same fixture with the tracked reference and asserts
every measured GPU texel matches within tolerance (so the GPU result cannot
silently drift from the tracked reference).

If RURIX_REQUIRE_REAL=1, an environment that would otherwise SKIP becomes a hard
failure (exit 1); otherwise SKIP exits 0, matching the repo GPU-smoke policy.
"""
from __future__ import annotations

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
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "taa_resolve"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "taa_resolve.dxil"
RTS0 = ARTIFACTS / "taa_resolve.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "taa_resolve_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_SCRIPT = PASS_DIR / "generate_math_parity_evidence.py"
EVIDENCE_OUT = PASS_DIR / "real_d3d12_dispatch_smoke.json"
WORK = ROOT / "target" / "grx012_d3d12_dispatch_smoke"

SUBJECT = "grx012_taa_resolve_real_d3d12_dispatch_smoke"

# RGB tolerance at the taa_resolve math-parity caliber (division/sqrt-heavy, so
# a touch looser than the ssao add/mul/div chain). The recorded max_abs_diff
# shows the real gap. Alpha is written as exactly 1.0.
VALUE_TOLERANCE = 3e-3
ALPHA_TOLERANCE = 1e-6


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
    """Import the tracked taa_resolve math-parity reference implementation so
    the Python check uses the SAME reference the pass ships."""
    spec = importlib.util.spec_from_file_location(
        "grx012_taa_resolve_math_parity", MATH_PARITY_SCRIPT
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
    """Return None when the descriptor layout matches the tracked GRX-012
    taa_resolve resource mapping, otherwise a human-readable mismatch reason."""
    resources = layout.get("resources")
    expected = [
        ("color_buffer", "t", 0, "texture2d"),
        ("depth_buffer", "t", 1, "texture2d"),
        ("velocity_buffer", "t", 2, "texture2d"),
        ("last_velocity_buffer", "t", 3, "texture2d"),
        ("history_buffer", "t", 4, "texture2d"),
        ("output_buffer", "u", 0, "rwtexture2d"),
    ]
    if not isinstance(resources, list) or len(resources) != 6:
        return "descriptor layout does not declare exactly 6 resources"
    for i, (name, cls, reg, kind) in enumerate(expected):
        r = resources[i]
        if not (isinstance(r, dict) and r.get("name") == name and r.get("class") == cls
                and r.get("register") == reg and r.get("binding_kind") == kind):
            return f"resource[{i}] is not {name} {cls}{reg} (binding_kind {kind})"
    if layout.get("root_signature_parameters") != 2:
        return "root_signature_parameters != 2"
    if layout.get("root_constants") != 5:
        return "root_constants != 5"
    mapping = layout.get("grx012_mapping")
    if not isinstance(mapping, dict):
        return "missing grx012_mapping"
    if mapping.get("root_constant_bytes") != 28 or mapping.get("root_constant_dwords") != 7:
        return "root constant block is not 28 bytes / 7 dwords"
    names = [entry.get("name") for entry in layout.get("root_constant_layout", []) if isinstance(entry, dict)]
    if names != ["source_width", "source_height", "disocclusion_threshold", "variance_dynamic", "reserved0"]:
        return "root_constant_layout names do not match the taa_resolve contract"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx012-d3d12-dispatch-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx012-d3d12-dispatch-smoke] SKIP {msg}(降级 SKIP,退出 0)")
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
    print(f"[grx012-d3d12-dispatch-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


# ---------------------------------------------------------------------------
# Real D3D12 compute-dispatch harness (C++/MSVC), compiled on demand.
#
# argv: <dxil> <rts0> <out_bin> [dxil.dll]
# Exit codes: 0 = success, 1 = fail, 2 = skip (no adapter / runtime).
#
# Root signature is created DIRECTLY from the Rurix RTS0 bytes, the compute PSO
# from the Rurix DXIL container, and the descriptor table is bound per the
# descriptor layout:
#   root param 0 = 7-dword (28-byte) b0 root constants
#   root param 1 = descriptor table [ SRV t0..t4 (color/depth/velocity/
#                  last_velocity/history), UAV u0 (output) ]
#
# The 16x16 fixture textures are filled with the SAME deterministic synthetic
# patterns as generate_math_parity_evidence.py (build_dispatch_fixture). The
# harness writes the full float32 RGBA output readback to <out_bin> (tight
# W*H*4 row-major) and the Python side re-verifies every texel against the
# tracked taa_resolve_frame reference.
# ---------------------------------------------------------------------------
HARNESS_CPP = r"""#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>

#include <dxcapi.h>

using Microsoft::WRL::ComPtr;

static const UINT FW = 16, FH = 16;
static const float VARIANCE_DYNAMIC = 1.0f;

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
static D3D12_RESOURCE_DESC buffer_desc(UINT64 bytes) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    d.Width = bytes;
    d.Height = 1;
    d.DepthOrArraySize = 1;
    d.MipLevels = 1;
    d.Format = DXGI_FORMAT_UNKNOWN;
    d.SampleDesc.Count = 1;
    d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    return d;
}
static D3D12_RESOURCE_DESC tex2d_desc(UINT w, UINT h, DXGI_FORMAT fmt, D3D12_RESOURCE_FLAGS flags) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    d.Width = w;
    d.Height = h;
    d.DepthOrArraySize = 1;
    d.MipLevels = 1;
    d.Format = fmt;
    d.SampleDesc.Count = 1;
    d.Flags = flags;
    return d;
}
static std::string narrow(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string out((size_t)std::max(n - 1, 0), '\0');
    if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr);
    return out;
}

// Deterministic synthetic inputs — bit-identical to
// generate_math_parity_evidence.py syn_* (int mod then float divide).
static void syn_color(int x, int y, float* o) {
    o[0] = (float)(((x * 7 + y * 13) % 97)) / 96.0f;
    o[1] = (float)(((x * 11 + y * 5) % 89)) / 88.0f;
    o[2] = (float)(((x * 3 + y * 17) % 83)) / 82.0f;
    o[3] = 0.0f;
}
static float syn_depth(int x, int y) {
    return (float)(((x * 5 + y * 9) % 64)) / 64.0f;
}
static void syn_velocity(int x, int y, float* o) {
    o[0] = (float)((((x * 2 + y) % 7) - 3)) / 256.0f;
    o[1] = (float)((((x + y * 2) % 7) - 3)) / 256.0f;
}
static void syn_last_velocity(int x, int y, float* o) {
    o[0] = (float)((((x + y * 3) % 5) - 2)) / 256.0f;
    o[1] = (float)((((x * 3 + y) % 5) - 2)) / 256.0f;
}
static void syn_history(int x, int y, float* o) {
    o[0] = (float)(((x * 13 + y * 7) % 91)) / 90.0f;
    o[1] = (float)(((x * 17 + y * 3) % 79)) / 78.0f;
    o[2] = (float)(((x * 5 + y * 11) % 73)) / 72.0f;
    o[3] = 0.0f;
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

// Create a DEFAULT-heap texture, fill it from an upload buffer via `fill`, and
// record a copy + transition to NON_PIXEL_SHADER_RESOURCE on `cmd`.
static bool make_input_texture(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,
                               UINT w, UINT h, DXGI_FORMAT fmt, UINT comps,
                               void (*fill)(int, int, float*),
                               ComPtr<ID3D12Resource>& tex,
                               ComPtr<ID3D12Resource>& upload,
                               const char* label) {
    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    auto upload_heap = heap_props(D3D12_HEAP_TYPE_UPLOAD);
    auto desc = tex2d_desc(w, h, fmt, D3D12_RESOURCE_FLAG_NONE);
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &desc,
                                               D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                               IID_PPV_ARGS(&tex)))) {
        std::fprintf(stderr, "RXGD_DISPATCH: fail CreateCommittedResource(%s)\n", label);
        return false;
    }
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT fp = {};
    UINT rows = 0; UINT64 row_size = 0, total = 0;
    device->GetCopyableFootprints(&desc, 0, 1, 0, &fp, &rows, &row_size, &total);
    auto up_desc = buffer_desc(total);
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
    for (UINT y = 0; y < h; ++y) {
        float* row = reinterpret_cast<float*>(p + fp.Offset + (SIZE_T)y * fp.Footprint.RowPitch);
        for (UINT x = 0; x < w; ++x) {
            float px[4] = {0, 0, 0, 0};
            fill((int)x, (int)y, px);
            for (UINT c = 0; c < comps; ++c) row[x * comps + c] = px[c];
        }
    }
    upload->Unmap(0, nullptr);
    D3D12_TEXTURE_COPY_LOCATION cdst = {};
    cdst.pResource = tex.Get();
    cdst.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    cdst.SubresourceIndex = 0;
    D3D12_TEXTURE_COPY_LOCATION csrc = {};
    csrc.pResource = upload.Get();
    csrc.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    csrc.PlacedFootprint = fp;
    cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
    D3D12_RESOURCE_BARRIER b = {};
    b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    b.Transition.pResource = tex.Get();
    b.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST;
    b.Transition.StateAfter = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
    b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &b);
    return true;
}

// wrappers with the (int,int,float*) signature make_input_texture expects
static void fill_color(int x, int y, float* o) { syn_color(x, y, o); }
static void fill_depth(int x, int y, float* o) { o[0] = syn_depth(x, y); }
static void fill_velocity(int x, int y, float* o) { syn_velocity(x, y, o); }
static void fill_last_velocity(int x, int y, float* o) { syn_last_velocity(x, y, o); }
static void fill_history(int x, int y, float* o) { syn_history(x, y, o); }

int wmain(int argc, wchar_t** argv) {
    if (argc < 4 || argc > 5) return fail_msg("usage: harness dxil rts0 out_bin [dxil.dll]");
    bool ok_dxil = false, ok_rts0 = false;
    auto dxil = read_file(argv[1], &ok_dxil);
    const auto rts0 = read_file(argv[2], &ok_rts0);
    if (!ok_dxil || dxil.empty()) return fail_msg("read dxil");
    if (!ok_rts0 || rts0.empty()) return fail_msg("read rts0");
    const wchar_t* out_bin = argv[3];
    const wchar_t* dxil_dll = (argc >= 5) ? argv[4] : nullptr;

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

    // Five input SRV textures (t0..t4) + one output UAV (u0), all 16x16 float32.
    ComPtr<ID3D12Resource> t_color, t_depth, t_vel, t_last, t_hist;
    ComPtr<ID3D12Resource> u_color, u_depth, u_vel, u_last, u_hist;
    if (!make_input_texture(device.Get(), cmd.Get(), FW, FH, DXGI_FORMAT_R32G32B32A32_FLOAT, 4, fill_color, t_color, u_color, "color"))
        return 1;
    if (!make_input_texture(device.Get(), cmd.Get(), FW, FH, DXGI_FORMAT_R32_FLOAT, 1, fill_depth, t_depth, u_depth, "depth"))
        return 1;
    if (!make_input_texture(device.Get(), cmd.Get(), FW, FH, DXGI_FORMAT_R32G32_FLOAT, 2, fill_velocity, t_vel, u_vel, "velocity"))
        return 1;
    if (!make_input_texture(device.Get(), cmd.Get(), FW, FH, DXGI_FORMAT_R32G32_FLOAT, 2, fill_last_velocity, t_last, u_last, "last_velocity"))
        return 1;
    if (!make_input_texture(device.Get(), cmd.Get(), FW, FH, DXGI_FORMAT_R32G32B32A32_FLOAT, 4, fill_history, t_hist, u_hist, "history"))
        return 1;

    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    auto out_desc = tex2d_desc(FW, FH, DXGI_FORMAT_R32G32B32A32_FLOAT, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    ComPtr<ID3D12Resource> out_tex;
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &out_desc,
                                               D3D12_RESOURCE_STATE_UNORDERED_ACCESS, nullptr,
                                               IID_PPV_ARGS(&out_tex))))
        return fail_msg("CreateCommittedResource(output)");

    // Descriptor heap: [SRV t0..t4, UAV u0]. The RTS0 table has the SRV range
    // (5 descriptors) then the UAV range (1 descriptor), APPEND-offset, so this
    // contiguous ordering matches.
    D3D12_DESCRIPTOR_HEAP_DESC hd = {};
    hd.NumDescriptors = 6;
    hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap;
    if (FAILED(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap))))
        return fail_msg("CreateDescriptorHeap(cbv_srv_uav)");
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    D3D12_CPU_DESCRIPTOR_HANDLE cpu = heap->GetCPUDescriptorHandleForHeapStart();

    auto make_srv = [&](ID3D12Resource* res, DXGI_FORMAT fmt, UINT slot) {
        D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
        srv.Format = fmt;
        srv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
        srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
        srv.Texture2D.MipLevels = 1;
        D3D12_CPU_DESCRIPTOR_HANDLE h = cpu; h.ptr += (SIZE_T)slot * inc;
        device->CreateShaderResourceView(res, &srv, h);
    };
    make_srv(t_color.Get(), DXGI_FORMAT_R32G32B32A32_FLOAT, 0);
    make_srv(t_depth.Get(), DXGI_FORMAT_R32_FLOAT, 1);
    make_srv(t_vel.Get(), DXGI_FORMAT_R32G32_FLOAT, 2);
    make_srv(t_last.Get(), DXGI_FORMAT_R32G32_FLOAT, 3);
    make_srv(t_hist.Get(), DXGI_FORMAT_R32G32B32A32_FLOAT, 4);
    {
        D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
        uav.Format = DXGI_FORMAT_R32G32B32A32_FLOAT;
        uav.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
        D3D12_CPU_DESCRIPTOR_HANDLE h = cpu; h.ptr += (SIZE_T)5 * inc;
        device->CreateUnorderedAccessView(out_tex.Get(), nullptr, &uav, h);
    }

    // Readback buffer for the output UAV.
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT dfp = {};
    UINT drows = 0; UINT64 drow_size = 0, dtotal = 0;
    device->GetCopyableFootprints(&out_desc, 0, 1, 0, &dfp, &drows, &drow_size, &dtotal);
    auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);
    auto rb_desc = buffer_desc(dtotal);
    ComPtr<ID3D12Resource> readback;
    if (FAILED(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                               D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                               IID_PPV_ARGS(&readback))))
        return fail_msg("CreateCommittedResource(readback)");

    // Bind + dispatch.
    cmd->SetComputeRootSignature(root.Get());
    ID3D12DescriptorHeap* heaps[] = {heap.Get()};
    cmd->SetDescriptorHeaps(1, heaps);
    uint32_t rc[7];
    uint64_t sw = FW, sh = FH;
    float disocclusion_threshold = 0.1f / (float)std::max(FW, FH);
    float reserved0 = 0.0f;
    std::memcpy(&rc[0], &sw, sizeof(uint64_t));                    // source_width  (dwords 0..1)
    std::memcpy(&rc[2], &sh, sizeof(uint64_t));                    // source_height (dwords 2..3)
    std::memcpy(&rc[4], &disocclusion_threshold, sizeof(float));  // dword 4
    std::memcpy(&rc[5], &VARIANCE_DYNAMIC, sizeof(float));        // dword 5
    std::memcpy(&rc[6], &reserved0, sizeof(float));               // dword 6
    cmd->SetComputeRoot32BitConstants(0, 7, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetPipelineState(pso.Get());
    const UINT gx = (FW + 7) / 8, gy = (FH + 7) / 8, gz = 1;
    cmd->Dispatch(gx, gy, gz);

    D3D12_RESOURCE_BARRIER db = {};
    db.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    db.Transition.pResource = out_tex.Get();
    db.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    db.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
    db.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &db);
    D3D12_TEXTURE_COPY_LOCATION cdst = {};
    cdst.pResource = readback.Get();
    cdst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    cdst.PlacedFootprint = dfp;
    D3D12_TEXTURE_COPY_LOCATION csrc = {};
    csrc.pResource = out_tex.Get();
    csrc.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    csrc.SubresourceIndex = 0;
    cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
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

    // Read back and write the tight W*H*4 float32 output to out_bin.
    uint8_t* mapped = nullptr;
    D3D12_RANGE range = {0, (SIZE_T)dtotal};
    if (FAILED(readback->Map(0, &range, reinterpret_cast<void**>(&mapped)))) return fail_msg("Map readback");
    std::vector<float> flat((size_t)FW * FH * 4);
    uint32_t checksum = 2166136261u;
    for (UINT y = 0; y < FH; ++y) {
        const uint8_t* rowp = mapped + dfp.Offset + (SIZE_T)y * dfp.Footprint.RowPitch;
        for (UINT x = 0; x < FW; ++x) {
            const uint8_t* px = rowp + (SIZE_T)x * 16;
            std::memcpy(&flat[((size_t)y * FW + x) * 4], px, 16);
            for (int b = 0; b < 16; ++b) { checksum ^= px[b]; checksum *= 16777619u; }
        }
    }
    readback->Unmap(0, nullptr);

    std::ofstream of(out_bin, std::ios::binary);
    if (!of) return fail_msg("open out_bin");
    of.write(reinterpret_cast<const char*>(flat.data()), (std::streamsize)(flat.size() * sizeof(float)));
    of.close();
    if (!of) return fail_msg("write out_bin");

    const UINT sxs[4] = {0u, FW / 2u, FW - 1u, 1u};
    const UINT sys[4] = {0u, FH / 2u, FH - 1u, FH - 2u};
    for (int s = 0; s < 4; ++s) {
        const float* p = &flat[((size_t)sys[s] * FW + sxs[s]) * 4];
        std::printf("RXGD_DISPATCH: sample x=%u y=%u obs=%g,%g,%g,%g\n",
                    sxs[s], sys[s], p[0], p[1], p[2], p[3]);
    }
    std::printf("RXGD_DISPATCH: ok adapter=\"%s\" dispatch=%u,%u,%u fence=%llu dst=%ux%u checksum=0x%08x\n",
                narrow(chosen_desc.Description).c_str(), gx, gy, gz,
                (unsigned long long)fence_done, FW, FH, checksum);
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
    parsed: dict = {"samples": []}
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("RXGD_DISPATCH: experimental_shader_models="):
            parsed["experimental_shader_models"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_DISPATCH: dxil_signed_for_load="):
            parsed["dxil_signed_for_load"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_DISPATCH: sample "):
            entry: dict = {}
            for token in ("x=", "y=", "obs="):
                idx = line.find(token)
                if idx >= 0:
                    entry[token.rstrip("=")] = line[idx + len(token):].split(" ", 1)[0]
            parsed["samples"].append(entry)
        elif line.startswith("RXGD_DISPATCH: ok "):
            for token in ("dispatch=", "fence=", "dst=", "checksum="):
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


def compare_readback(parity, out_bin: Path) -> dict:
    """Compare every GPU output texel against the tracked taa_resolve_frame
    reference on the dispatch fixture."""
    w, h = parity.DISPATCH_WIDTH, parity.DISPATCH_HEIGHT
    color, depth, velocity, last_velocity, history = parity.build_dispatch_fixture(w, h)
    dth = parity.dispatch_disocclusion_threshold(w, h)
    ref = parity.taa_resolve_frame(w, h, color, depth, velocity, last_velocity, history,
                                   dth, parity.DISPATCH_VARIANCE_DYNAMIC)
    raw = out_bin.read_bytes()
    expected_len = w * h * 4 * 4
    if len(raw) != expected_len:
        return {"match": False, "reason": f"output binary size {len(raw)} != {expected_len}"}
    obs = struct.unpack(f"<{w * h * 4}f", raw)
    max_abs = 0.0
    mismatched = 0
    worst = None
    for y in range(h):
        for x in range(w):
            base = (y * w + x) * 4
            r = ref[y][x]
            o = obs[base:base + 4]
            drgb = max(abs(o[c] - r[c]) for c in range(3))
            da = abs(o[3] - r[3])
            if drgb > max_abs:
                max_abs = drgb
                worst = {"x": x, "y": y, "observed": list(o), "reference": r}
            if drgb > VALUE_TOLERANCE or da > ALPHA_TOLERANCE:
                mismatched += 1
    return {
        "match": mismatched == 0,
        "max_abs_diff": max_abs,
        "mismatched_texels": mismatched,
        "total_texels": w * h,
        "value_tolerance": VALUE_TOLERANCE,
        "alpha_tolerance": ALPHA_TOLERANCE,
        "worst_texel": worst,
    }


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
        return fail("cannot read taa_resolve_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "taa_resolve",
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
            "GRX-012 standalone real D3D12 dispatch smoke evidence only. A success "
            "flips real_d3d12_dispatch_recorded/cpu_reference_match true (the fields "
            "the GRX-012 gate reads) but keeps runtime_state=fallback_only and "
            "real_gpu_pass=false; it is not a Godot runtime pass, visual, perf, or "
            "measured-telemetry claim."
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
    if parity is None or not hasattr(parity, "taa_resolve_frame"):
        return fail(
            "cannot import the tracked generate_math_parity_evidence.py reference "
            "implementation (taa_resolve_frame) for the CPU cross-check"
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
    cpp = WORK / "taa_resolve_dispatch_harness.cpp"
    exe = WORK / "taa_resolve_dispatch_harness.exe"
    out_bin = WORK / "taa_output.bin"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")

    built, build_log = compile_harness(vcvars, cpp, exe, include_dir)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 dispatch harness 失败(可能缺 Windows SDK D3D12 头/库)",
                    extra={"build_log_tail": build_log})

    if out_bin.exists():
        out_bin.unlink()
    p = run([str(exe), str(DXIL), str(RTS0), str(out_bin), str(dxil_dll)], cwd=WORK)
    output = (p.stdout + p.stderr).strip()
    print(output)
    parsed = parse_harness_output(output)
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
        return fail("real D3D12 taa_resolve dispatch smoke failed",
                    extra={"device": device_info, "exit_code": p.returncode, "stdout": output})

    comparison = compare_readback(parity, out_bin)
    if not comparison.get("match"):
        return fail(
            "GPU-observed output texels did not match the tracked taa_resolve_frame reference",
            extra={"device": device_info, "cpu_reference_comparison": comparison, "stdout": output},
        )

    dispatch = {
        "dimensions": parsed.get("dispatch"),
        "fence_completed_value": parsed.get("fence"),
        "dst_shape": parsed.get("dst"),
        "readback_checksum": parsed.get("checksum"),
        "samples": parsed.get("samples"),
    }
    cpu_reference = {
        "reference_impl": (
            "spike/godot-rurix/passes/taa_resolve/generate_math_parity_evidence.py "
            "taa_resolve_frame (imported and compared over every texel in Python)"
        ),
        "fixture": {
            "width": parity.DISPATCH_WIDTH,
            "height": parity.DISPATCH_HEIGHT,
            "variance_dynamic": parity.DISPATCH_VARIANCE_DYNAMIC,
            "math_parity_case": "taa_resolve_dispatch_fixture_16x16",
        },
        "comparison": comparison,
    }
    write_evidence(
        "success",
        extra={
            "real_d3d12_dispatch_recorded": True,
            "cpu_reference_match": True,
            "device": device_info,
            "dispatch": dispatch,
            "cpu_reference": cpu_reference,
            "checks": {
                "artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "root_signature_create_from_rurix_rts0": True,
                "compute_pso_from_rurix_dxil": True,
                "six_resources_bound_from_layout": True,
                "dispatch_executed": True,
                "fence_completed": True,
                "output_uav_readback": True,
                "all_output_texels_match_cpu_reference": comparison.get("match") is True,
            },
            "stdout": output,
        },
    )
    print(f"[grx012-d3d12-dispatch-smoke] PASS measured real D3D12 dispatch; "
          f"adapter={device_info['adapter']} max_abs_diff={comparison.get('max_abs_diff')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
