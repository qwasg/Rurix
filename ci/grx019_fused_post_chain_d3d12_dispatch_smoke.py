#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-019: standalone real Windows D3D12 dispatch smoke for fused_post_chain.

fused_post_chain is a TEXTURE pass that fuses the luminance-WRITE (EMA) segment
and the tonemap (LINEAR + sRGB) segment into one dispatch. It binds THREE
Texture2D SRVs (t0 ``src_color``, t1 ``lum_source``, t2 ``prev_luminance``) and
TWO RWTexture2D UAVs (u0 ``dst_color``, u1 ``dst_luminance``) with a 64-byte
(16-dword) b0 whose four leading dimension fields are ``uint2`` i64 pairs (low
dword carries the value, high dword is zero).

This harness uploads the tracked ``dispatch_fixture_case`` inputs (full-precision
R32/R32G32B32A32 float textures), records one dispatch on a real D3D12 device,
reads back both output textures, and compares them to the CPU reference
``fused_frame`` within the pass's documented ABSOLUTE tolerance
(``MAX_ABS_ERROR_TOLERANCE`` = 3e-3; the tonemap has a divide + sRGB ``pow`` so a
small drift is expected). The alpha channel is a straight passthrough of the
input and is compared near-exactly. Measured smoke evidence only; not a Godot
runtime pass / visual / perf claim. Real device/queue only (SKIP otherwise).
Three tracked digests verified against the offline evidence. The b0 carries i64
``uint2`` dimension pairs but the kernel reads only the low dword, so the plain
``cs_6_0`` container runs on any FL 11_0 device (no Int64 device feature gate).
If RURIX_REQUIRE_REAL=1 a SKIP becomes a hard failure.
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
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "fused_post_chain"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "fused_post_chain.dxil"
RTS0 = ARTIFACTS / "fused_post_chain.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "fused_post_chain_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_SCRIPT = PASS_DIR / "generate_math_parity_evidence.py"
EVIDENCE_OUT = PASS_DIR / "real_d3d12_dispatch_smoke.json"
WORK = ROOT / "target" / "grx019_d3d12_dispatch_smoke"

SUBJECT = "grx019_fused_post_chain_real_d3d12_dispatch_smoke"

RGBA_STRIDE = 16  # R32G32B32A32_FLOAT
R32_STRIDE = 4    # R32_FLOAT
ALPHA_TOLERANCE = 1.0e-6


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
    spec = importlib.util.spec_from_file_location(
        "grx019_fused_post_chain_math_parity", MATH_PARITY_SCRIPT
    )
    if spec is None or spec.loader is None:
        return None
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except Exception:  # noqa: BLE001
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
    resources = layout.get("resources")
    expected = [
        ("src_color", "t", 0, "texture2d"),
        ("lum_source", "t", 1, "texture2d"),
        ("prev_luminance", "t", 2, "texture2d"),
        ("dst_color", "u", 0, "rwtexture2d"),
        ("dst_luminance", "u", 1, "rwtexture2d"),
    ]
    if not isinstance(resources, list) or len(resources) != 5:
        return "descriptor layout does not declare exactly 5 resources"
    for i, (name, cls, reg, kind) in enumerate(expected):
        r = resources[i]
        if not (isinstance(r, dict) and r.get("name") == name and r.get("class") == cls
                and r.get("register") == reg and r.get("binding_kind") == kind):
            return f"resource[{i}] is not {name} {cls}{reg} (binding_kind {kind})"
    if layout.get("root_signature_parameters") != 2:
        return "root_signature_parameters != 2"
    if layout.get("root_constants") != 12:
        return "root_constants != 12"
    mapping = layout.get("grx019_mapping")
    if not isinstance(mapping, dict):
        return "missing grx019_mapping"
    if mapping.get("root_constant_bytes") != 64 or mapping.get("root_constant_dwords") != 16:
        return "root constant block is not 64 bytes / 16 dwords"
    if mapping.get("requires_64bit_integer_shader_capability") is not True:
        return "grx019_mapping must record requires_64bit_integer_shader_capability=true"
    if mapping.get("resource_count") != 5 or mapping.get("srv_count") != 3 or mapping.get("uav_count") != 2:
        return "grx019_mapping resource/srv/uav counts do not match 5/3/2"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx019-d3d12-dispatch-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx019-d3d12-dispatch-smoke] SKIP {msg}(降级 SKIP,退出 0)")
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
    print(f"[grx019-d3d12-dispatch-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


def build_case_payload(parity) -> dict:
    """The single tracked dispatch fixture. Inputs are full-precision float
    textures; expected outputs come from the CPU reference ``fused_frame``."""
    width = parity.DISPATCH_WIDTH
    height = parity.DISPATCH_HEIGHT
    lum_w = parity.DISPATCH_LUM_W
    lum_h = parity.DISPATCH_LUM_H
    prev = parity.DISPATCH_PREV
    consts = dict(parity.BASE_CONSTANTS)
    color = parity.build_color(width, height)          # [h][w] of (r,g,b,a)
    lum_grid = parity.build_lum(lum_w, lum_h, 1.0)      # [lh][lw] of f32
    ldr_grid, lum_out, cur, avg, exposure_eff = parity.fused_frame(
        width, height, color, lum_grid, lum_w, lum_h, prev, consts, False
    )

    # Flatten inputs to tight little-endian float bytes.
    src_color = bytearray()
    for y in range(height):
        for x in range(width):
            r, g, b, a = color[y][x]
            src_color += struct.pack("<4f", r, g, b, a)
    lum_source = bytearray()
    for y in range(lum_h):
        for x in range(lum_w):
            lum_source += struct.pack("<f", lum_grid[y][x])
    prev_bytes = struct.pack("<f", prev)

    # Expected dst_color (tight RGBA float rows) + dst_luminance (single float).
    expected_color = []
    for y in range(height):
        for x in range(width):
            expected_color.append(tuple(ldr_grid[y][x]))
    expected_lum = lum_out

    # 64-byte b0: 4 uint2 i64 pairs (low, high=0) + 8 f32.
    b0 = struct.pack("<II", width, 0)
    b0 += struct.pack("<II", height, 0)
    b0 += struct.pack("<II", lum_w, 0)
    b0 += struct.pack("<II", lum_h, 0)
    b0 += struct.pack(
        "<8f",
        consts["max_luminance"],
        consts["min_luminance"],
        consts["exposure_adjust"],
        consts["exposure"],
        consts["white"],
        consts["luminance_multiplier"],
        0.0,  # first_frame
        consts["auto_exposure_scale"],
    )
    if len(b0) != 64:
        raise ValueError(f"b0 is {len(b0)} bytes, expected 64")

    params = struct.pack("<IIII", width, height, lum_w, lum_h)
    params += b0 + bytes(src_color) + bytes(lum_source) + prev_bytes
    return {
        "width": width,
        "height": height,
        "lum_w": lum_w,
        "lum_h": lum_h,
        "params": params,
        "expected_color": expected_color,
        "expected_lum": expected_lum,
        "tolerance": parity.MAX_ABS_ERROR_TOLERANCE,
    }


def compare_outputs(payload: dict, out_bin: Path) -> dict:
    width = payload["width"]
    height = payload["height"]
    tol = payload["tolerance"]
    raw = out_bin.read_bytes()
    color_bytes = width * height * RGBA_STRIDE
    if len(raw) != color_bytes + R32_STRIDE:
        return {"match": False, "reason": f"output size {len(raw)} != {color_bytes + R32_STRIDE}"}
    color = struct.unpack(f"<{width * height * 4}f", raw[:color_bytes])
    lum = struct.unpack("<f", raw[color_bytes:])[0]

    max_rgb_err = 0.0
    max_alpha_err = 0.0
    worst = None
    for idx, exp in enumerate(payload["expected_color"]):
        o = color[idx * 4:idx * 4 + 4]
        for c in range(3):
            err = abs(o[c] - exp[c])
            if err > max_rgb_err:
                max_rgb_err = err
                if err > tol:
                    worst = {"pixel": idx, "channel": c, "observed": o[c], "expected": exp[c], "err": err}
        aerr = abs(o[3] - exp[3])
        max_alpha_err = max(max_alpha_err, aerr)
    lum_err = abs(lum - payload["expected_lum"])
    match = (max_rgb_err <= tol) and (max_alpha_err <= ALPHA_TOLERANCE) and (lum_err <= tol)
    return {
        "match": match,
        "max_rgb_abs_error": max_rgb_err,
        "max_alpha_abs_error": max_alpha_err,
        "dst_luminance_observed": lum,
        "dst_luminance_expected": payload["expected_lum"],
        "dst_luminance_abs_error": lum_err,
        "tolerance": tol,
        "alpha_tolerance": ALPHA_TOLERANCE,
        "worst": worst,
    }


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

static int fail_hr(const char* what, HRESULT hr) { std::fprintf(stderr, "RXGD_DISPATCH: fail %s hr=0x%08lx\n", what, (unsigned long)hr); return 1; }
static int fail_msg(const char* what) { std::fprintf(stderr, "RXGD_DISPATCH: fail %s\n", what); return 1; }
static int skip_msg(const char* what) { std::fprintf(stderr, "RXGD_DISPATCH: skip %s\n", what); return 2; }

static std::vector<uint8_t> read_file(const wchar_t* path, bool* ok) {
    *ok = false; std::ifstream f(path, std::ios::binary); if (!f) return {};
    f.seekg(0, std::ios::end); const auto n = f.tellg(); if (n <= 0) return {};
    f.seekg(0, std::ios::beg); std::vector<uint8_t> data((size_t)n);
    f.read(reinterpret_cast<char*>(data.data()), n); if (!f) return {}; *ok = true; return data;
}
static D3D12_HEAP_PROPERTIES heap_props(D3D12_HEAP_TYPE type) { D3D12_HEAP_PROPERTIES hp = {}; hp.Type = type; hp.CreationNodeMask = 1; hp.VisibleNodeMask = 1; return hp; }
static std::string narrow(const wchar_t* s) { int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr); std::string out((size_t)std::max(n - 1, 0), '\0'); if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr); return out; }

struct MemBlob : public IDxcBlob {
    LONG m_ref; void* m_ptr; SIZE_T m_size; MemBlob(void* p, SIZE_T s) : m_ref(1), m_ptr(p), m_size(s) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** ppv) override { if (!ppv) return E_POINTER; if (riid == __uuidof(IUnknown) || riid == __uuidof(IDxcBlob)) { *ppv = static_cast<IDxcBlob*>(this); AddRef(); return S_OK; } *ppv = nullptr; return E_NOINTERFACE; }
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
    if (FAILED(create(CLSID_DxcValidator, __uuidof(IDxcValidator), reinterpret_cast<void**>(&validator))) || !validator) { *err = "DxcCreateInstance failed"; return false; }
    MemBlob blob(dxil.data(), dxil.size()); IDxcOperationResult* result = nullptr;
    HRESULT hr = validator->Validate(&blob, DxcValidatorFlags_InPlaceEdit, &result); bool ok = false;
    if (SUCCEEDED(hr) && result) { HRESULT st = E_FAIL; result->GetStatus(&st); ok = SUCCEEDED(st); if (!ok) *err = "validator rejected the DXIL container"; } else { *err = "Validate failed"; }
    if (result) result->Release(); validator->Release(); return ok;
}

static D3D12_RESOURCE_DESC tex_desc(DXGI_FORMAT fmt, UINT w, UINT h, D3D12_RESOURCE_FLAGS flags) {
    D3D12_RESOURCE_DESC d = {}; d.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D; d.Width = w; d.Height = h; d.DepthOrArraySize = 1;
    d.MipLevels = 1; d.Format = fmt; d.SampleDesc.Count = 1; d.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN; d.Flags = flags; return d;
}
static D3D12_RESOURCE_DESC buf_desc(UINT64 bytes) {
    D3D12_RESOURCE_DESC d = {}; d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; d.Width = bytes; d.Height = 1; d.DepthOrArraySize = 1;
    d.MipLevels = 1; d.Format = DXGI_FORMAT_UNKNOWN; d.SampleDesc.Count = 1; d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; return d;
}

int wmain(int argc, wchar_t** argv) {
    if (argc < 5 || argc > 6) return fail_msg("usage: harness dxil rts0 params out [dxil.dll]");
    bool ok = false;
    auto dxil = read_file(argv[1], &ok); if (!ok) return fail_msg("read dxil");
    auto rts0 = read_file(argv[2], &ok); if (!ok) return fail_msg("read rts0");
    auto params = read_file(argv[3], &ok); if (!ok) return fail_msg("read params");
    const wchar_t* out_bin = argv[4];
    const wchar_t* dxil_dll = (argc >= 6) ? argv[5] : nullptr;
    if (params.size() < 16 + 64) return fail_msg("params too small");

    UINT width = 0, height = 0, lum_w = 0, lum_h = 0;
    std::memcpy(&width, params.data() + 0, 4);
    std::memcpy(&height, params.data() + 4, 4);
    std::memcpy(&lum_w, params.data() + 8, 4);
    std::memcpy(&lum_h, params.data() + 12, 4);
    const uint8_t* b0 = params.data() + 16;
    const UINT src_bytes = width * height * 16u;
    const UINT lum_bytes = lum_w * lum_h * 4u;
    const UINT prev_bytes = 4u;
    const uint8_t* src_data = params.data() + 16 + 64;
    const uint8_t* lum_data = src_data + src_bytes;
    const uint8_t* prev_data = lum_data + lum_bytes;
    if (params.size() != (size_t)(16 + 64 + src_bytes + lum_bytes + prev_bytes)) return fail_msg("params size mismatch");

    { static const GUID kExp = D3D12ExperimentalShaderModels; bool ex = SUCCEEDED(D3D12EnableExperimentalFeatures(1, &kExp, nullptr, nullptr)); std::printf("RXGD_DISPATCH: experimental_shader_models=%s\n", ex ? "on" : "off"); }
    std::string se; bool signed_ok = sign_dxil_in_place(dxil, dxil_dll, &se);
    std::printf("RXGD_DISPATCH: dxil_signed_for_load=%s\n", signed_ok ? "yes" : "no");

    ComPtr<IDXGIFactory6> factory; if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) return skip_msg("no DXGI factory");
    ComPtr<IDXGIAdapter1> chosen; DXGI_ADAPTER_DESC1 cd = {}; SIZE_T best = 0;
    for (UINT i = 0;; ++i) { ComPtr<IDXGIAdapter1> a; HRESULT e = factory->EnumAdapters1(i, &a); if (e == DXGI_ERROR_NOT_FOUND) break; if (FAILED(e)) break; DXGI_ADAPTER_DESC1 d = {}; a->GetDesc1(&d); if (d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) continue; if (SUCCEEDED(D3D12CreateDevice(a.Get(), D3D_FEATURE_LEVEL_11_0, __uuidof(ID3D12Device), nullptr)) && d.DedicatedVideoMemory >= best) { best = d.DedicatedVideoMemory; chosen = a; cd = d; } }
    if (!chosen) return skip_msg("no hardware D3D12 adapter");
    ComPtr<ID3D12Device> device; if (FAILED(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)))) return skip_msg("D3D12CreateDevice failed");
    std::printf("RXGD_DISPATCH: adapter=\"%s\"\n", narrow(cd.Description).c_str());

    ComPtr<ID3D12RootSignature> root; if (FAILED(device->CreateRootSignature(0, rts0.data(), rts0.size(), IID_PPV_ARGS(&root)))) return fail_msg("CreateRootSignature");
    D3D12_COMPUTE_PIPELINE_STATE_DESC pd = {}; pd.pRootSignature = root.Get(); pd.CS = {dxil.data(), dxil.size()};
    ComPtr<ID3D12PipelineState> pso; if (FAILED(device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&pso)))) return fail_msg("CreateComputePipelineState");

    D3D12_COMMAND_QUEUE_DESC qd = {}; qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue; if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)))) return fail_msg("CreateCommandQueue");
    ComPtr<ID3D12CommandAllocator> alloc; if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc)))) return fail_msg("CreateCommandAllocator");
    ComPtr<ID3D12GraphicsCommandList> cmd; if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(), nullptr, IID_PPV_ARGS(&cmd)))) return fail_msg("CreateCommandList");

    auto dh = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    // Upload a texture from tight little-endian pixel data.
    auto make_input_tex = [&](DXGI_FORMAT fmt, UINT w, UINT h, UINT bpp, const uint8_t* data,
                              ComPtr<ID3D12Resource>& tex, ComPtr<ID3D12Resource>& up) -> bool {
        auto td = tex_desc(fmt, w, h, D3D12_RESOURCE_FLAG_NONE);
        if (FAILED(device->CreateCommittedResource(&dh, D3D12_HEAP_FLAG_NONE, &td, D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&tex)))) return false;
        D3D12_PLACED_SUBRESOURCE_FOOTPRINT fp = {}; UINT rows = 0; UINT64 rowsize = 0, total = 0;
        device->GetCopyableFootprints(&td, 0, 1, 0, &fp, &rows, &rowsize, &total);
        auto uh = heap_props(D3D12_HEAP_TYPE_UPLOAD); auto ud = buf_desc(total);
        if (FAILED(device->CreateCommittedResource(&uh, D3D12_HEAP_FLAG_NONE, &ud, D3D12_RESOURCE_STATE_GENERIC_READ, nullptr, IID_PPV_ARGS(&up)))) return false;
        uint8_t* p = nullptr; D3D12_RANGE e = {0, 0};
        if (FAILED(up->Map(0, &e, reinterpret_cast<void**>(&p)))) return false;
        for (UINT y = 0; y < h; ++y) std::memcpy(p + fp.Offset + (SIZE_T)y * fp.Footprint.RowPitch, data + (SIZE_T)y * w * bpp, (SIZE_T)w * bpp);
        up->Unmap(0, nullptr);
        D3D12_TEXTURE_COPY_LOCATION cdst = {}; cdst.pResource = tex.Get(); cdst.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX; cdst.SubresourceIndex = 0;
        D3D12_TEXTURE_COPY_LOCATION csrc = {}; csrc.pResource = up.Get(); csrc.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT; csrc.PlacedFootprint = fp;
        cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
        D3D12_RESOURCE_BARRIER b = {}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION; b.Transition.pResource = tex.Get();
        b.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST; b.Transition.StateAfter = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE; b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &b);
        return true;
    };
    auto make_output_tex = [&](DXGI_FORMAT fmt, UINT w, UINT h, ComPtr<ID3D12Resource>& tex) -> bool {
        auto td = tex_desc(fmt, w, h, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        return SUCCEEDED(device->CreateCommittedResource(&dh, D3D12_HEAP_FLAG_NONE, &td, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, nullptr, IID_PPV_ARGS(&tex)));
    };

    ComPtr<ID3D12Resource> src_tex, src_up, lum_tex, lum_up, prev_tex, prev_up, dst_color, dst_lum;
    if (!make_input_tex(DXGI_FORMAT_R32G32B32A32_FLOAT, width, height, 16, src_data, src_tex, src_up)) return fail_msg("make src_color");
    if (!make_input_tex(DXGI_FORMAT_R32_FLOAT, lum_w, lum_h, 4, lum_data, lum_tex, lum_up)) return fail_msg("make lum_source");
    if (!make_input_tex(DXGI_FORMAT_R32_FLOAT, 1, 1, 4, prev_data, prev_tex, prev_up)) return fail_msg("make prev_luminance");
    if (!make_output_tex(DXGI_FORMAT_R32G32B32A32_FLOAT, width, height, dst_color)) return fail_msg("make dst_color");
    if (!make_output_tex(DXGI_FORMAT_R32_FLOAT, 1, 1, dst_lum)) return fail_msg("make dst_luminance");

    D3D12_DESCRIPTOR_HEAP_DESC hd = {}; hd.NumDescriptors = 5; hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV; hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap; if (FAILED(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap)))) return fail_msg("CreateDescriptorHeap");
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    auto cpu = [&](UINT i) { D3D12_CPU_DESCRIPTOR_HANDLE h = heap->GetCPUDescriptorHandleForHeapStart(); h.ptr += (SIZE_T)i * inc; return h; };
    auto make_srv = [&](ID3D12Resource* r, DXGI_FORMAT fmt, UINT i) { D3D12_SHADER_RESOURCE_VIEW_DESC s = {}; s.Format = fmt; s.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D; s.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING; s.Texture2D.MipLevels = 1; device->CreateShaderResourceView(r, &s, cpu(i)); };
    auto make_uav = [&](ID3D12Resource* r, DXGI_FORMAT fmt, UINT i) { D3D12_UNORDERED_ACCESS_VIEW_DESC u = {}; u.Format = fmt; u.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D; device->CreateUnorderedAccessView(r, nullptr, &u, cpu(i)); };
    make_srv(src_tex.Get(), DXGI_FORMAT_R32G32B32A32_FLOAT, 0);
    make_srv(lum_tex.Get(), DXGI_FORMAT_R32_FLOAT, 1);
    make_srv(prev_tex.Get(), DXGI_FORMAT_R32_FLOAT, 2);
    make_uav(dst_color.Get(), DXGI_FORMAT_R32G32B32A32_FLOAT, 3);
    make_uav(dst_lum.Get(), DXGI_FORMAT_R32_FLOAT, 4);

    ID3D12DescriptorHeap* heaps[] = {heap.Get()}; cmd->SetDescriptorHeaps(1, heaps);
    cmd->SetComputeRootSignature(root.Get());
    cmd->SetPipelineState(pso.Get());
    uint32_t rc[16]; std::memcpy(rc, b0, 64);
    cmd->SetComputeRoot32BitConstants(0, 16, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, heap->GetGPUDescriptorHandleForHeapStart());
    const UINT gx = std::max<UINT>((width + 7u) / 8u, 1u);
    const UINT gy = std::max<UINT>((height + 7u) / 8u, 1u);
    cmd->Dispatch(gx, gy, 1);

    // Readback both output textures.
    auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
    auto read_tex = [&](ID3D12Resource* tex, DXGI_FORMAT fmt, UINT w, UINT h, UINT bpp, ComPtr<ID3D12Resource>& rb, D3D12_PLACED_SUBRESOURCE_FOOTPRINT& fp) -> bool {
        auto td = tex_desc(fmt, w, h, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        UINT rows = 0; UINT64 rowsize = 0, total = 0;
        device->GetCopyableFootprints(&td, 0, 1, 0, &fp, &rows, &rowsize, &total);
        auto rd = buf_desc(total);
        if (FAILED(device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &rd, D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&rb)))) return false;
        D3D12_RESOURCE_BARRIER b = {}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION; b.Transition.pResource = tex;
        b.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS; b.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE; b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &b);
        D3D12_TEXTURE_COPY_LOCATION cdst = {}; cdst.pResource = rb.Get(); cdst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT; cdst.PlacedFootprint = fp;
        D3D12_TEXTURE_COPY_LOCATION csrc = {}; csrc.pResource = tex; csrc.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX; csrc.SubresourceIndex = 0;
        cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
        (void)bpp;
        return true;
    };
    ComPtr<ID3D12Resource> rb_color, rb_lum;
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT fp_color = {}, fp_lum = {};
    if (!read_tex(dst_color.Get(), DXGI_FORMAT_R32G32B32A32_FLOAT, width, height, 16, rb_color, fp_color)) return fail_msg("readback dst_color setup");
    if (!read_tex(dst_lum.Get(), DXGI_FORMAT_R32_FLOAT, 1, 1, 4, rb_lum, fp_lum)) return fail_msg("readback dst_luminance setup");
    if (FAILED(cmd->Close())) return fail_msg("Close command list");

    ID3D12CommandList* lists[] = {cmd.Get()}; queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence; if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) return fail_msg("CreateFence");
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (FAILED(queue->Signal(fence.Get(), 1))) return fail_msg("Signal fence");
    if (fence->GetCompletedValue() < 1) { fence->SetEventOnCompletion(1, ev); WaitForSingleObject(ev, INFINITE); }
    CloseHandle(ev);
    if (fence->GetCompletedValue() < 1) return fail_msg("fence did not complete");

    std::vector<uint8_t> out;
    { // dst_color tight rows.
        uint8_t* m = nullptr; D3D12_RANGE r = {0, 0}; UINT64 total = (UINT64)fp_color.Offset + (UINT64)fp_color.Footprint.RowPitch * height;
        r.End = (SIZE_T)total; if (FAILED(rb_color->Map(0, &r, reinterpret_cast<void**>(&m)))) return fail_msg("Map dst_color");
        for (UINT y = 0; y < height; ++y) { const uint8_t* rowp = m + fp_color.Offset + (SIZE_T)y * fp_color.Footprint.RowPitch; size_t off = out.size(); out.resize(off + (size_t)width * 16); std::memcpy(out.data() + off, rowp, (size_t)width * 16); }
        rb_color->Unmap(0, nullptr);
    }
    { // dst_luminance single texel.
        uint8_t* m = nullptr; D3D12_RANGE r = {0, (SIZE_T)(fp_lum.Offset + 4)}; if (FAILED(rb_lum->Map(0, &r, reinterpret_cast<void**>(&m)))) return fail_msg("Map dst_luminance");
        size_t off = out.size(); out.resize(off + 4); std::memcpy(out.data() + off, m + fp_lum.Offset, 4);
        rb_lum->Unmap(0, nullptr);
    }
    std::ofstream of(out_bin, std::ios::binary); if (!of) return fail_msg("open out_bin");
    of.write(reinterpret_cast<const char*>(out.data()), (std::streamsize)out.size()); of.close(); if (!of) return fail_msg("write out_bin");

    float lum_first = 0.0f; std::memcpy(&lum_first, out.data() + out.size() - 4, 4);
    std::printf("RXGD_DISPATCH: ok adapter=\"%s\" dispatch=%u,%u,1 fence=%llu size=%ux%u dst_luminance=%g\n",
                narrow(cd.Description).c_str(), gx, gy, (unsigned long long)fence->GetCompletedValue(), width, height, lum_first);
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
            for token in ("dispatch=", "fence=", "dst_luminance="):
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
        return fail("cannot read fused_post_chain_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    hashes_match = (
        dxil_sha == offline_digests["dxil"]
        and rts0_sha == offline_digests["root_signature"]
        and layout_sha == offline_digests["descriptor_layout"]
    )
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "fused_post_chain",
        "segment": "standalone_dispatch_smoke",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "cpu_reference_match": False,
        "artifacts": {
            "dxil": {"path": str(DXIL.relative_to(ROOT)).replace("\\", "/"), "sha256": dxil_sha},
            "root_signature": {"path": str(RTS0.relative_to(ROOT)).replace("\\", "/"), "sha256": rts0_sha},
            "descriptor_layout": {"path": str(DESCRIPTOR_LAYOUT.relative_to(ROOT)).replace("\\", "/"), "sha256": layout_sha},
        },
        "offline_evidence": {
            "path": str(OFFLINE_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
            "dxil_sha256": offline_digests["dxil"],
            "root_signature_sha256": offline_digests["root_signature"],
            "descriptor_layout_sha256": offline_digests["descriptor_layout"],
        },
        "artifact_hashes_match_offline_evidence": hashes_match,
        "note": (
            "GRX-019 standalone real D3D12 fused_post_chain dispatch smoke evidence "
            "only (3 Texture2D SRVs + 2 RWTexture2D UAVs). Float comparison within the "
            "pass's documented absolute tolerance. A success flips "
            "real_d3d12_dispatch_recorded/cpu_reference_match true but keeps "
            "runtime_state=fallback_only and real_gpu_pass=false."
        ),
    }

    if not hashes_match:
        return fail("artifact SHA-256 does not match tracked offline compile evidence",
                    extra={"observed": {"dxil": dxil_sha, "rts0": rts0_sha, "layout": layout_sha},
                           "offline": offline_digests})

    layout_issue = descriptor_layout_matches_resource_mapping(layout)
    if layout_issue is not None:
        return fail(f"descriptor layout / resource mapping mismatch: {layout_issue}")

    parity = load_math_parity_reference()
    if parity is None or not all(hasattr(parity, n) for n in ("fused_frame", "build_color", "build_lum",
                                                              "BASE_CONSTANTS", "DISPATCH_WIDTH", "MAX_ABS_ERROR_TOLERANCE")):
        return fail("cannot import the tracked generate_math_parity_evidence.py reference (fused_frame / build_color / ...)")

    vcvars = locate_vcvars64()
    if vcvars is None:
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64)")
    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip("未找到含 dxil.dll 的签名 DXC pin(set RURIX_DXC_DIR)")
    include_dir = locate_dxcapi_include(dxc_dir)
    if include_dir is None:
        return skip(f"未在 {dxc_dir} 附近找到 dxcapi.h")
    dxil_dll = dxc_dir / "dxil.dll"

    WORK.mkdir(parents=True, exist_ok=True)
    cpp = WORK / "fused_post_chain_dispatch_harness.cpp"
    exe = WORK / "fused_post_chain_dispatch_harness.exe"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")
    built, build_log = compile_harness(vcvars, cpp, exe, include_dir)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 dispatch harness 失败", extra={"build_log_tail": build_log})

    payload = build_case_payload(parity)
    params_bin = WORK / "params_dispatch_fixture.bin"
    out_bin = WORK / "out_dispatch_fixture.bin"
    params_bin.write_bytes(payload["params"])
    if out_bin.exists():
        out_bin.unlink()
    p = run([str(exe), str(DXIL), str(RTS0), str(params_bin), str(out_bin), str(dxil_dll)], cwd=WORK)
    output = (p.stdout + p.stderr).strip()
    print("--- case dispatch_fixture ---")
    print(output)
    parsed = parse_harness_output(output)
    device_info = {
        "adapter": parsed.get("adapter"),
        "experimental_shader_models": parsed.get("experimental_shader_models"),
        "dxil_signed_for_load": parsed.get("dxil_signed_for_load"),
        "dxil_validator": str(dxil_dll).replace("\\", "/"),
    }
    if p.returncode == 2:
        return skip("no real D3D12 device harness available", extra={"device": device_info, "stdout": output})
    if p.returncode != 0 or "RXGD_DISPATCH: ok" not in output or not out_bin.is_file():
        return fail("real D3D12 fused_post_chain dispatch smoke failed",
                    extra={"device": device_info, "exit_code": p.returncode, "stdout": output})

    comparison = compare_outputs(payload, out_bin)
    if not comparison.get("match"):
        return fail("GPU-observed fused_post_chain output exceeded the tracked absolute tolerance",
                    extra={"device": device_info, "comparison": comparison})

    write_evidence(
        "success",
        extra={
            "real_d3d12_dispatch_recorded": True,
            "cpu_reference_match": True,
            "device": device_info,
            "cpu_reference": {
                "reference_impl": (
                    "spike/godot-rurix/passes/fused_post_chain/generate_math_parity_evidence.py "
                    "fused_frame (imported; dst_color RGB + dst_luminance within MAX_ABS_ERROR_TOLERANCE, "
                    "alpha near-exact)"
                ),
                "tolerance": payload["tolerance"],
                "alpha_tolerance": ALPHA_TOLERANCE,
                "case": {
                    "case_id": "dispatch_fixture",
                    "extent": [payload["width"], payload["height"]],
                    "lum_source_extent": [payload["lum_w"], payload["lum_h"]],
                    "dispatch": parsed.get("dispatch"),
                    "fence_completed_value": parsed.get("fence"),
                    "comparison": comparison,
                },
            },
            "checks": {
                "three_artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "three_srv_two_uav_textures_bound_from_layout": True,
                "i64_uint2_dimension_pairs_low_dword_carried": True,
                "dispatch_executed": True,
                "fence_completed": True,
                "both_uav_texture_readback": True,
                "outputs_within_absolute_tolerance": True,
            },
        },
    )
    print(f"[grx019-d3d12-dispatch-smoke] PASS measured real D3D12 fused dispatch; "
          f"adapter={device_info.get('adapter')} max_rgb_err={comparison.get('max_rgb_abs_error'):.2e} "
          f"lum_err={comparison.get('dst_luminance_abs_error'):.2e} tol={payload['tolerance']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
