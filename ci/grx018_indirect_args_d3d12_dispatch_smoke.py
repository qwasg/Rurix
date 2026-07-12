#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-018: standalone real Windows D3D12 dispatch smoke for the indirect_args pass.

indirect_args is a PAIRED write + validate pass: the write kernel fills a
per-surface indirect draw command buffer (clamping each surface's instance count
to ``max_instance_count``), and the validate kernel re-inspects the produced
command buffer and writes a diagnostic buffer (``mismatch_count``,
``clamp_trigger_count``, per-surface bitmask). Both kernels share one root
signature and a 3-buffer descriptor table (SRV t0 ``src_survivor_counts`` +
RWStructuredBuffer UAV u0 ``dst_command_buffer`` + RWStructuredBuffer UAV u1
``dst_validation``) with a 176-byte (44-dword) b0.

This harness records write -> UAV barrier -> validate -> readback in ONE submit
on a real D3D12 device and compares both output buffers against the tracked
``generate_math_parity_evidence.py`` references **exactly** (pure u32 word math,
tolerance ZERO). It runs BOTH legs required by the GRX-018 §5 contract:

  * the CLEAN legs (cases 1-5): write then validate; the validation buffer must
    match the reference (including the legitimate clamp-trigger counter of the
    ``clamp_triggered_producer_violation`` case), and the produced command buffer
    must match the write reference.
  * the CORRUPTED-STAGING RED leg (``validation_detects_corruption``): the command
    buffer is seeded with a deliberately corrupted staging buffer and validate is
    run alone; the validation buffer MUST report a NON-ZERO mismatch count,
    proving validate flags corruption rather than silently reporting clean.

Measured smoke evidence only; not a Godot runtime pass / visual / perf claim.
Real device/queue only (SKIP otherwise). Four tracked digests (write DXIL,
validate DXIL, shared RTS0, descriptor layout) verified against the offline
evidence. If RURIX_REQUIRE_REAL=1 a SKIP becomes a hard failure.
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
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "indirect_args"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "indirect_args.dxil"
VALIDATE_DXIL = ARTIFACTS / "indirect_args_validate.dxil"
RTS0 = ARTIFACTS / "indirect_args.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "indirect_args_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_SCRIPT = PASS_DIR / "generate_math_parity_evidence.py"
EVIDENCE_OUT = PASS_DIR / "real_d3d12_dispatch_smoke.json"
WORK = ROOT / "target" / "grx018_d3d12_dispatch_smoke"

SUBJECT = "grx018_indirect_args_real_d3d12_dispatch_smoke"

COMMAND_STRIDE_DWORDS = 5
MAX_SURFACES = 8
VALUE_TOLERANCE = 0


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
        "grx018_indirect_args_math_parity", MATH_PARITY_SCRIPT
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
    keys = ("dxil", "dxil_validate", "root_signature", "descriptor_layout")
    out: dict[str, str | None] = {k: None for k in keys}
    if isinstance(artifacts, dict):
        for key in keys:
            entry = artifacts.get(key)
            if isinstance(entry, dict):
                sha = entry.get("sha256")
                if isinstance(sha, str):
                    out[key] = sha
    return out


def descriptor_layout_matches_resource_mapping(layout: dict) -> str | None:
    resources = layout.get("resources")
    expected = [
        ("src_survivor_counts", "t", 0, "structured_buffer"),
        ("dst_command_buffer", "u", 0, "rwstructured_buffer"),
        ("dst_validation", "u", 1, "rwstructured_buffer"),
    ]
    if not isinstance(resources, list) or len(resources) != 3:
        return "descriptor layout does not declare exactly 3 resources"
    for i, (name, cls, reg, kind) in enumerate(expected):
        r = resources[i]
        if not (isinstance(r, dict) and r.get("name") == name and r.get("class") == cls
                and r.get("register") == reg and r.get("binding_kind") == kind):
            return f"resource[{i}] is not {name} {cls}{reg} (binding_kind {kind})"
    if layout.get("root_signature_parameters") != 2:
        return "root_signature_parameters != 2"
    if layout.get("root_constants") != 44:
        return "root_constants != 44"
    mapping = layout.get("grx018_mapping")
    if not isinstance(mapping, dict):
        return "missing grx018_mapping"
    if mapping.get("root_constant_bytes") != 176 or mapping.get("root_constant_dwords") != 44:
        return "root constant block is not 176 bytes / 44 dwords"
    if mapping.get("requires_64bit_integer_shader_capability") is not False:
        return "grx018_mapping must record requires_64bit_integer_shader_capability=false"
    if mapping.get("command_block_stride_dwords") != COMMAND_STRIDE_DWORDS:
        return "grx018_mapping command_block_stride_dwords is not 5"
    if mapping.get("max_surfaces") != MAX_SURFACES:
        return "grx018_mapping max_surfaces is not 8"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx018-d3d12-dispatch-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx018-d3d12-dispatch-smoke] SKIP {msg}(降级 SKIP,退出 0)")
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
    print(f"[grx018-d3d12-dispatch-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


def make_cases(parity) -> list[dict]:
    """Six coherent fixtures mirroring the generator's tracked case set. build_case
    computes the CPU reference from these consts, so the GPU run (using the same b0
    + survivor buffer) is a self-consistent cross-check of the kernels."""
    mt = parity.make_templates
    specs = [
        ("single_surface_basic",
         dict(surface_count=1, max_instance_count=64, survivor_count_word_offset=0,
              templates=mt([(36, 0, 0, 0)])),
         [17], None),
        ("multi_surface_shared_count",
         dict(surface_count=4, max_instance_count=64, survivor_count_word_offset=0,
              templates=mt([(36, 0, 0, 0), (72, 6, 10, 0), (108, 12, 20, 0), (144, 18, 30, 0)])),
         [9], None),
        ("clamp_triggered_producer_violation",
         dict(surface_count=2, max_instance_count=64, survivor_count_word_offset=0,
              templates=mt([(36, 0, 0, 0), (72, 6, 10, 0)])),
         [100], None),
        ("zero_survivors",
         dict(surface_count=3, max_instance_count=128, survivor_count_word_offset=0,
              templates=mt([(36, 0, 0, 0), (72, 6, 10, 0), (108, 12, 20, 0)])),
         [0], None),
        ("max_surfaces_nonzero_statics_and_offset",
         dict(surface_count=8, max_instance_count=4096, survivor_count_word_offset=3,
              templates=mt([(16777216 + s * 7, s * 3, s * 5, s * 2) for s in range(8)])),
         [111, 222, 333, 2048, 555], None),
        ("validation_detects_corruption",
         dict(surface_count=4, max_instance_count=64, survivor_count_word_offset=0,
              templates=mt([(36, 0, 0, 0), (72, 6, 10, 0), (108, 12, 20, 0), (144, 18, 30, 0)])),
         [9], {1 * 5 + 0: 4501, 2 * 5 + 1: 999}),
    ]
    cases = []
    for case_id, consts, survivor_buffer, corrupt in specs:
        doc = parity.build_case(case_id, consts, survivor_buffer, corrupt=corrupt)
        cases.append({"case_id": case_id, "consts": consts, "survivor_buffer": survivor_buffer,
                      "corrupt": corrupt, "doc": doc})
    return cases


def build_b0(parity, consts: dict) -> bytes:
    tw = parity.template_words(consts["templates"])
    b0 = struct.pack("<4I", consts["surface_count"], consts["max_instance_count"],
                     consts["survivor_count_word_offset"], 0)
    b0 += struct.pack("<40I", *tw)
    if len(b0) != 176:
        raise ValueError(f"b0 is {len(b0)} bytes, expected 176")
    return b0


def build_case_payload(parity, case: dict) -> dict:
    consts = case["consts"]
    doc = case["doc"]
    surface_count = consts["surface_count"]
    survivor_buffer = case["survivor_buffer"]
    b0 = build_b0(parity, consts)
    survivor_bytes = struct.pack(f"<{len(survivor_buffer)}I", *survivor_buffer)
    command_words = surface_count * COMMAND_STRIDE_DWORDS
    command_bytes = command_words * 4
    validation_words = 2 + surface_count
    validation_bytes = validation_words * 4
    corrupted = case["corrupt"] is not None
    # mode 0 = write+validate (clean); mode 1 = seed-corrupted-command + validate.
    mode = 1 if corrupted else 0
    if corrupted:
        command_initial = struct.pack(f"<{command_words}I", *doc["validation_input_command_buffer"])
    else:
        command_initial = b"\x00" * command_bytes
    params = struct.pack("<IIII", mode, len(survivor_bytes), command_bytes, validation_bytes)
    params += b0 + survivor_bytes + command_initial
    return {
        "case_id": case["case_id"],
        "mode": mode,
        "surface_count": surface_count,
        "command_bytes": command_bytes,
        "validation_bytes": validation_bytes,
        "expected_command": list(doc["cpu_expected_command_buffer"]),
        "expected_validation": list(doc["cpu_expected_validation"]["words"]),
        "expected_mismatch_count": doc["cpu_expected_validation"]["mismatch_count"],
        "expected_clamp_trigger_count": doc["cpu_expected_validation"]["clamp_trigger_count"],
        "params": params,
    }


def compare_outputs(payload: dict, out_bin: Path) -> dict:
    raw = out_bin.read_bytes()
    command_bytes = payload["command_bytes"]
    validation_bytes = payload["validation_bytes"]
    if len(raw) != command_bytes + validation_bytes:
        return {"match": False, "reason": f"output size {len(raw)} != {command_bytes + validation_bytes}"}
    cmd_words = list(struct.unpack(f"<{command_bytes // 4}I", raw[:command_bytes]))
    val_words = list(struct.unpack(f"<{validation_bytes // 4}I", raw[command_bytes:]))
    val_match = val_words == payload["expected_validation"]
    if payload["mode"] == 0:
        cmd_match = cmd_words == payload["expected_command"]
    else:
        # Red leg: the command buffer was seeded (corrupted) and not re-written;
        # only the validation output is the meaningful measured result.
        cmd_match = True
    return {
        "match": val_match and cmd_match,
        "command_match": cmd_match,
        "validation_match": val_match,
        "observed_validation": val_words,
        "expected_validation": payload["expected_validation"],
        "observed_command": cmd_words,
        "value_tolerance": VALUE_TOLERANCE,
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
static const UINT WORD_STRIDE = 4u;

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
static D3D12_RESOURCE_DESC buffer_desc(UINT64 bytes, D3D12_RESOURCE_FLAGS flags) {
    D3D12_RESOURCE_DESC d = {}; d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; d.Width = bytes; d.Height = 1; d.DepthOrArraySize = 1;
    d.MipLevels = 1; d.Format = DXGI_FORMAT_UNKNOWN; d.SampleDesc.Count = 1; d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; d.Flags = flags; return d;
}
static std::string narrow(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string out((size_t)std::max(n - 1, 0), '\0');
    if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr); return out;
}
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
    if (FAILED(create(CLSID_DxcValidator, __uuidof(IDxcValidator), reinterpret_cast<void**>(&validator))) || !validator) { *err = "DxcCreateInstance(CLSID_DxcValidator) failed"; return false; }
    MemBlob blob(dxil.data(), dxil.size()); IDxcOperationResult* result = nullptr;
    HRESULT hr = validator->Validate(&blob, DxcValidatorFlags_InPlaceEdit, &result); bool ok = false;
    if (SUCCEEDED(hr) && result) { HRESULT st = E_FAIL; result->GetStatus(&st); ok = SUCCEEDED(st); if (!ok) *err = "validator rejected the DXIL container"; } else { *err = "IDxcValidator::Validate failed"; }
    if (result) result->Release(); validator->Release(); return ok;
}
static bool make_buffer(ID3D12Device* device, ID3D12GraphicsCommandList* cmd, UINT64 bytes, D3D12_RESOURCE_FLAGS flags,
                        D3D12_RESOURCE_STATES after, const uint8_t* data, ComPtr<ID3D12Resource>& buf, ComPtr<ID3D12Resource>& upload, const char* label) {
    auto dh = heap_props(D3D12_HEAP_TYPE_DEFAULT); auto desc = buffer_desc(bytes, flags);
    D3D12_RESOURCE_STATES initial = data ? D3D12_RESOURCE_STATE_COPY_DEST : after;
    if (FAILED(device->CreateCommittedResource(&dh, D3D12_HEAP_FLAG_NONE, &desc, initial, nullptr, IID_PPV_ARGS(&buf)))) { std::fprintf(stderr, "RXGD_DISPATCH: fail CreateCommittedResource(%s)\n", label); return false; }
    if (data) {
        auto uh = heap_props(D3D12_HEAP_TYPE_UPLOAD); auto ud = buffer_desc(bytes, D3D12_RESOURCE_FLAG_NONE);
        if (FAILED(device->CreateCommittedResource(&uh, D3D12_HEAP_FLAG_NONE, &ud, D3D12_RESOURCE_STATE_GENERIC_READ, nullptr, IID_PPV_ARGS(&upload)))) { std::fprintf(stderr, "RXGD_DISPATCH: fail upload(%s)\n", label); return false; }
        uint8_t* p = nullptr; D3D12_RANGE e = {0, 0};
        if (FAILED(upload->Map(0, &e, reinterpret_cast<void**>(&p)))) { std::fprintf(stderr, "RXGD_DISPATCH: fail Map(%s)\n", label); return false; }
        std::memcpy(p, data, (size_t)bytes); upload->Unmap(0, nullptr);
        cmd->CopyBufferRegion(buf.Get(), 0, upload.Get(), 0, bytes);
        D3D12_RESOURCE_BARRIER b = {}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION; b.Transition.pResource = buf.Get();
        b.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST; b.Transition.StateAfter = after; b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &b);
    }
    return true;
}
static void srv(ID3D12Device* d, ID3D12Resource* r, UINT bytes, D3D12_CPU_DESCRIPTOR_HANDLE h) {
    D3D12_SHADER_RESOURCE_VIEW_DESC s = {}; s.Format = DXGI_FORMAT_UNKNOWN; s.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
    s.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING; s.Buffer.FirstElement = 0;
    s.Buffer.NumElements = std::max<UINT>(bytes / WORD_STRIDE, 1u); s.Buffer.StructureByteStride = WORD_STRIDE; s.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
    d->CreateShaderResourceView(r, &s, h);
}
static void uav(ID3D12Device* d, ID3D12Resource* r, UINT bytes, D3D12_CPU_DESCRIPTOR_HANDLE h) {
    D3D12_UNORDERED_ACCESS_VIEW_DESC u = {}; u.Format = DXGI_FORMAT_UNKNOWN; u.ViewDimension = D3D12_UAV_DIMENSION_BUFFER;
    u.Buffer.FirstElement = 0; u.Buffer.NumElements = std::max<UINT>(bytes / WORD_STRIDE, 1u); u.Buffer.StructureByteStride = WORD_STRIDE; u.Buffer.CounterOffsetInBytes = 0; u.Buffer.Flags = D3D12_BUFFER_UAV_FLAG_NONE;
    d->CreateUnorderedAccessView(r, nullptr, &u, h);
}

// argv: write_dxil validate_dxil rts0 params out [dxil.dll]
int wmain(int argc, wchar_t** argv) {
    if (argc < 6 || argc > 7) return fail_msg("usage: harness write_dxil validate_dxil rts0 params out [dxil.dll]");
    bool ok = false;
    auto write_dxil = read_file(argv[1], &ok); if (!ok) return fail_msg("read write dxil");
    auto validate_dxil = read_file(argv[2], &ok); if (!ok) return fail_msg("read validate dxil");
    auto rts0 = read_file(argv[3], &ok); if (!ok) return fail_msg("read rts0");
    auto params = read_file(argv[4], &ok); if (!ok) return fail_msg("read params");
    const wchar_t* out_bin = argv[5];
    const wchar_t* dxil_dll = (argc >= 7) ? argv[6] : nullptr;
    if (params.size() < 16 + 176) return fail_msg("params too small");

    UINT mode = 0, survivor_bytes = 0, command_bytes = 0, validation_bytes = 0;
    std::memcpy(&mode, params.data() + 0, 4);
    std::memcpy(&survivor_bytes, params.data() + 4, 4);
    std::memcpy(&command_bytes, params.data() + 8, 4);
    std::memcpy(&validation_bytes, params.data() + 12, 4);
    const uint8_t* b0 = params.data() + 16;
    const uint8_t* survivor_data = params.data() + 16 + 176;
    const uint8_t* command_data = survivor_data + survivor_bytes;
    if (params.size() != (size_t)(16 + 176 + survivor_bytes + command_bytes)) return fail_msg("params size mismatch");

    { static const GUID kExp = D3D12ExperimentalShaderModels; bool ex = SUCCEEDED(D3D12EnableExperimentalFeatures(1, &kExp, nullptr, nullptr)); std::printf("RXGD_DISPATCH: experimental_shader_models=%s\n", ex ? "on" : "off"); }
    std::string se; bool s1 = sign_dxil_in_place(write_dxil, dxil_dll, &se); bool s2 = sign_dxil_in_place(validate_dxil, dxil_dll, &se);
    std::printf("RXGD_DISPATCH: dxil_signed_for_load=%s\n", (s1 && s2) ? "yes" : "no");

    ComPtr<IDXGIFactory6> factory; if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) return skip_msg("no DXGI factory");
    ComPtr<IDXGIAdapter1> chosen; DXGI_ADAPTER_DESC1 cd = {}; SIZE_T best = 0;
    for (UINT i = 0;; ++i) { ComPtr<IDXGIAdapter1> a; HRESULT e = factory->EnumAdapters1(i, &a); if (e == DXGI_ERROR_NOT_FOUND) break; if (FAILED(e)) break; DXGI_ADAPTER_DESC1 d = {}; a->GetDesc1(&d); if (d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) continue; if (SUCCEEDED(D3D12CreateDevice(a.Get(), D3D_FEATURE_LEVEL_11_0, __uuidof(ID3D12Device), nullptr)) && d.DedicatedVideoMemory >= best) { best = d.DedicatedVideoMemory; chosen = a; cd = d; } }
    if (!chosen) return skip_msg("no hardware D3D12 adapter");
    ComPtr<ID3D12Device> device; if (FAILED(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)))) return skip_msg("D3D12CreateDevice failed");
    std::printf("RXGD_DISPATCH: adapter=\"%s\"\n", narrow(cd.Description).c_str());

    ComPtr<ID3D12RootSignature> root; if (FAILED(device->CreateRootSignature(0, rts0.data(), rts0.size(), IID_PPV_ARGS(&root)))) return fail_msg("CreateRootSignature");
    auto make_pso = [&](std::vector<uint8_t>& dxil, ComPtr<ID3D12PipelineState>& pso, const char* label) -> bool { D3D12_COMPUTE_PIPELINE_STATE_DESC pd = {}; pd.pRootSignature = root.Get(); pd.CS = {dxil.data(), dxil.size()}; if (FAILED(device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&pso)))) { std::fprintf(stderr, "RXGD_DISPATCH: fail pso %s\n", label); return false; } return true; };
    ComPtr<ID3D12PipelineState> pso_write, pso_validate;
    if (!make_pso(write_dxil, pso_write, "write")) return 1;
    if (!make_pso(validate_dxil, pso_validate, "validate")) return 1;

    D3D12_COMMAND_QUEUE_DESC qd = {}; qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue; if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)))) return fail_msg("CreateCommandQueue");
    ComPtr<ID3D12CommandAllocator> alloc; if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc)))) return fail_msg("CreateCommandAllocator");
    ComPtr<ID3D12GraphicsCommandList> cmd; if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(), nullptr, IID_PPV_ARGS(&cmd)))) return fail_msg("CreateCommandList");

    std::vector<uint8_t> zval(validation_bytes, 0);
    ComPtr<ID3D12Resource> sv_b, sv_u, cmd_b, cmd_u, val_b, val_u;
    if (!make_buffer(device.Get(), cmd.Get(), survivor_bytes, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, survivor_data, sv_b, sv_u, "src_survivor_counts")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), command_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, command_data, cmd_b, cmd_u, "dst_command_buffer")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), validation_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, zval.data(), val_b, val_u, "dst_validation")) return 1;

    D3D12_DESCRIPTOR_HEAP_DESC hd = {}; hd.NumDescriptors = 6; hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV; hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap; if (FAILED(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap)))) return fail_msg("CreateDescriptorHeap");
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    auto cpu = [&](UINT i) { D3D12_CPU_DESCRIPTOR_HANDLE h = heap->GetCPUDescriptorHandleForHeapStart(); h.ptr += (SIZE_T)i * inc; return h; };
    auto gpu = [&](UINT i) { D3D12_GPU_DESCRIPTOR_HANDLE h = heap->GetGPUDescriptorHandleForHeapStart(); h.ptr += (UINT64)i * inc; return h; };
    // Two identical tables (write at 0..2, validate at 3..5) over the same 3 buffers.
    for (UINT base = 0; base < 6; base += 3) {
        srv(device.Get(), sv_b.Get(), survivor_bytes, cpu(base + 0));
        uav(device.Get(), cmd_b.Get(), command_bytes, cpu(base + 1));
        uav(device.Get(), val_b.Get(), validation_bytes, cpu(base + 2));
    }
    ID3D12DescriptorHeap* heaps[] = {heap.Get()}; cmd->SetDescriptorHeaps(1, heaps);
    uint32_t rc[44]; std::memcpy(rc, b0, 176);

    if (mode == 0) {
        cmd->SetComputeRootSignature(root.Get());
        cmd->SetPipelineState(pso_write.Get());
        cmd->SetComputeRoot32BitConstants(0, 44, rc, 0);
        cmd->SetComputeRootDescriptorTable(1, gpu(0));
        cmd->Dispatch(1, 1, 1);
        D3D12_RESOURCE_BARRIER ub = {}; ub.Type = D3D12_RESOURCE_BARRIER_TYPE_UAV; ub.UAV.pResource = cmd_b.Get();
        cmd->ResourceBarrier(1, &ub);
    }
    cmd->SetComputeRootSignature(root.Get());
    cmd->SetPipelineState(pso_validate.Get());
    cmd->SetComputeRoot32BitConstants(0, 44, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, gpu(3));
    cmd->Dispatch(1, 1, 1);

    D3D12_RESOURCE_BARRIER tb[2] = {};
    tb[0].Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION; tb[0].Transition.pResource = cmd_b.Get();
    tb[0].Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS; tb[0].Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE; tb[0].Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    tb[1] = tb[0]; tb[1].Transition.pResource = val_b.Get();
    cmd->ResourceBarrier(2, tb);
    auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
    ComPtr<ID3D12Resource> rb_cmd, rb_val;
    { auto d = buffer_desc(command_bytes, D3D12_RESOURCE_FLAG_NONE); if (FAILED(device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &d, D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&rb_cmd)))) return fail_msg("readback command"); }
    { auto d = buffer_desc(validation_bytes, D3D12_RESOURCE_FLAG_NONE); if (FAILED(device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &d, D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&rb_val)))) return fail_msg("readback validation"); }
    cmd->CopyBufferRegion(rb_cmd.Get(), 0, cmd_b.Get(), 0, command_bytes);
    cmd->CopyBufferRegion(rb_val.Get(), 0, val_b.Get(), 0, validation_bytes);
    if (FAILED(cmd->Close())) return fail_msg("Close command list");

    ID3D12CommandList* lists[] = {cmd.Get()}; queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence; if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) return fail_msg("CreateFence");
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (FAILED(queue->Signal(fence.Get(), 1))) return fail_msg("Signal fence");
    if (fence->GetCompletedValue() < 1) { fence->SetEventOnCompletion(1, ev); WaitForSingleObject(ev, INFINITE); }
    CloseHandle(ev);
    if (fence->GetCompletedValue() < 1) return fail_msg("fence did not complete");

    std::vector<uint8_t> out((size_t)command_bytes + validation_bytes);
    { uint8_t* m = nullptr; D3D12_RANGE r = {0, (SIZE_T)command_bytes}; if (FAILED(rb_cmd->Map(0, &r, reinterpret_cast<void**>(&m)))) return fail_msg("Map command"); std::memcpy(out.data(), m, command_bytes); rb_cmd->Unmap(0, nullptr); }
    { uint8_t* m = nullptr; D3D12_RANGE r = {0, (SIZE_T)validation_bytes}; if (FAILED(rb_val->Map(0, &r, reinterpret_cast<void**>(&m)))) return fail_msg("Map validation"); std::memcpy(out.data() + command_bytes, m, validation_bytes); rb_val->Unmap(0, nullptr); }
    std::ofstream of(out_bin, std::ios::binary); if (!of) return fail_msg("open out_bin");
    of.write(reinterpret_cast<const char*>(out.data()), (std::streamsize)out.size()); of.close(); if (!of) return fail_msg("write out_bin");

    uint32_t mismatch = 0, clamp = 0;
    if (validation_bytes >= 8) { std::memcpy(&mismatch, out.data() + command_bytes, 4); std::memcpy(&clamp, out.data() + command_bytes + 4, 4); }
    std::printf("RXGD_DISPATCH: ok adapter=\"%s\" mode=%u command_bytes=%u validation_bytes=%u mismatch=%u clamp=%u\n",
                narrow(cd.Description).c_str(), mode, command_bytes, validation_bytes, mismatch, clamp);
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
            for token in ("mode=", "mismatch=", "clamp="):
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

    for path in (DXIL, VALIDATE_DXIL, RTS0, DESCRIPTOR_LAYOUT, OFFLINE_EVIDENCE, MATH_PARITY_SCRIPT):
        if not path.is_file():
            _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
            return fail(f"required artifact missing: {path.relative_to(ROOT)}")

    digests = {
        "dxil": sha256_file(DXIL),
        "dxil_validate": sha256_file(VALIDATE_DXIL),
        "root_signature": sha256_file(RTS0),
        "descriptor_layout": sha256_file(DESCRIPTOR_LAYOUT),
    }
    offline = load_json(OFFLINE_EVIDENCE)
    layout = load_json(DESCRIPTOR_LAYOUT)
    if offline is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read offline_compile_evidence.json")
    if layout is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read indirect_args_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    hashes_match = all(digests[k] == offline_digests[k] for k in digests)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "indirect_args",
        "segment": "standalone_dispatch_smoke",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "cpu_reference_match": False,
        "artifacts": {k: {"sha256": v} for k, v in digests.items()},
        "offline_evidence": {"path": str(OFFLINE_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
                             "digests": offline_digests},
        "artifact_hashes_match_offline_evidence": hashes_match,
        "note": (
            "GRX-018 standalone real D3D12 indirect_args write+validate dispatch smoke "
            "evidence only. Includes the mandatory corrupted-staging RED leg (validate "
            "must report a NON-ZERO mismatch count over a corrupted command buffer). A "
            "success flips real_d3d12_dispatch_recorded/cpu_reference_match true but keeps "
            "runtime_state=fallback_only and real_gpu_pass=false."
        ),
    }

    if not hashes_match:
        return fail("artifact SHA-256 does not match tracked offline compile evidence",
                    extra={"observed": digests, "offline": offline_digests})

    layout_issue = descriptor_layout_matches_resource_mapping(layout)
    if layout_issue is not None:
        return fail(f"descriptor layout / resource mapping mismatch: {layout_issue}")

    parity = load_math_parity_reference()
    if parity is None or not all(hasattr(parity, n) for n in ("build_case", "make_templates", "template_words",
                                                              "write_kernel_reference", "validate_kernel_reference")):
        return fail("cannot import the tracked generate_math_parity_evidence.py reference (build_case / make_templates / ...)")

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
    cpp = WORK / "indirect_args_dispatch_harness.cpp"
    exe = WORK / "indirect_args_dispatch_harness.exe"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")
    built, build_log = compile_harness(vcvars, cpp, exe, include_dir)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 dispatch harness 失败", extra={"build_log_tail": build_log})

    cases = make_cases(parity)
    device_info: dict = {}
    case_results: list[dict] = []
    all_match = True
    red_leg_confirmed = False
    for case in cases:
        payload = build_case_payload(parity, case)
        cid = payload["case_id"]
        params_bin = WORK / f"params_{cid}.bin"
        out_bin = WORK / f"out_{cid}.bin"
        params_bin.write_bytes(payload["params"])
        if out_bin.exists():
            out_bin.unlink()
        p = run([str(exe), str(DXIL), str(VALIDATE_DXIL), str(RTS0), str(params_bin), str(out_bin), str(dxil_dll)], cwd=WORK)
        output = (p.stdout + p.stderr).strip()
        print(f"--- case {cid} (mode={payload['mode']}) ---")
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
            return skip("no real D3D12 device harness available", extra={"device": device_info, "stdout": output})
        if p.returncode != 0 or "RXGD_DISPATCH: ok" not in output or not out_bin.is_file():
            return fail(f"real D3D12 indirect_args dispatch smoke failed for case {cid}",
                        extra={"device": device_info, "exit_code": p.returncode, "stdout": output})
        comparison = compare_outputs(payload, out_bin)
        if not comparison.get("match"):
            all_match = False
        observed_mismatch = int(parsed.get("mismatch", "0"))
        if cid == "validation_detects_corruption":
            # The RED leg: validate over a corrupted staging buffer MUST report a
            # non-zero mismatch count matching the reference.
            if observed_mismatch > 0 and comparison.get("validation_match"):
                red_leg_confirmed = True
            else:
                all_match = False
        case_results.append({
            "case_id": cid,
            "mode": payload["mode"],
            "surface_count": payload["surface_count"],
            "expected_mismatch_count": payload["expected_mismatch_count"],
            "expected_clamp_trigger_count": payload["expected_clamp_trigger_count"],
            "observed_mismatch_count": observed_mismatch,
            "observed_clamp_trigger_count": int(parsed.get("clamp", "0")),
            "comparison": comparison,
        })

    if not all_match:
        return fail("GPU-observed indirect_args output did not match the tracked reference exactly",
                    extra={"device": device_info, "cases": case_results})
    if not red_leg_confirmed:
        return fail("corrupted-staging RED leg did not report a non-zero mismatch count",
                    extra={"device": device_info, "cases": case_results})

    write_evidence(
        "success",
        extra={
            "real_d3d12_dispatch_recorded": True,
            "cpu_reference_match": True,
            "corrupted_staging_red_leg_confirmed": True,
            "device": device_info,
            "cpu_reference": {
                "reference_impl": (
                    "spike/godot-rurix/passes/indirect_args/generate_math_parity_evidence.py "
                    "write_kernel_reference + validate_kernel_reference (imported; command + "
                    "validation u32 words compared exactly)"
                ),
                "value_tolerance": VALUE_TOLERANCE,
                "cases": case_results,
            },
            "checks": {
                "four_artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "write_then_uav_barrier_then_validate": True,
                "clean_legs_command_and_validation_exact": True,
                "corrupted_staging_red_leg_nonzero_mismatch": True,
                "dispatch_executed": True,
                "fence_completed": True,
            },
        },
    )
    print(f"[grx018-d3d12-dispatch-smoke] PASS measured real D3D12 write+validate over "
          f"{len(case_results)} cases (incl. corrupted-staging RED leg); "
          f"adapter={device_info.get('adapter')} tolerance=0 (exact)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
