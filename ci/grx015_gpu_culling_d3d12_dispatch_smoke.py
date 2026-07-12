#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-015: standalone real Windows D3D12 dispatch smoke for the gpu_culling pass.

Template copy of ci/grx014_cluster_store_d3d12_dispatch_smoke.py adapted to the
GRX-015 gpu_culling package, which binds ONE StructuredBuffer SRV (t0
``src_transforms`` uint/float lanes) and TWO RWStructuredBuffer UAVs (u0
``dst_commands`` count-only ``InterlockedAdd``, u1 ``dst_visibility``
``InterlockedOr`` bitmask) with a 144-byte (36-dword) b0 root-constant block. It
proves the *offline* gpu_culling artifacts (the DXC-compiled DXIL container, the
Rurix-owned RTS0 root signature, and the descriptor layout) can complete **one
minimal compute dispatch on a real D3D12 device and command queue**, and
additionally verifies every measured GPU output word (both the command-buffer
count dwords and the visibility bitmask) against the CPU reference (the tracked
``generate_math_parity_evidence.py`` ``gpu_culling_reference``) **exactly** — the
kernel is pure integer word math, so the tolerance is ZERO. It produces measured
smoke evidence only. It does NOT mark the Godot runtime gpu_culling pass as
complete, make the bridge default to RXGD_STATUS_OK, or claim any FPS / visual
diff / measured fallback telemetry.

Discipline mirrors the GRX-014 dispatch smoke: real device/queue only (SKIP with
a concrete reason otherwise; SKIP never advances the ready gate), tracked
DXIL/RTS0/descriptor-layout artifacts used as-is with SHA-256 verified against
the tracked offline compile evidence, bindings created strictly from the
descriptor layout, and deterministic Python-built fixtures uploaded verbatim.
Unlike cluster_store, ``dst_commands`` is seeded with the tracked initial command
words (NOT blanket-zeroed) so the count-only writes land on the correct dwords;
``dst_visibility`` is explicitly zero-uploaded. If RURIX_REQUIRE_REAL=1 a SKIP
becomes a hard failure.
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
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "gpu_culling"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "gpu_culling.dxil"
RTS0 = ARTIFACTS / "gpu_culling.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "gpu_culling_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_SCRIPT = PASS_DIR / "generate_math_parity_evidence.py"
EVIDENCE_OUT = PASS_DIR / "real_d3d12_dispatch_smoke.json"
WORK = ROOT / "target" / "grx015_d3d12_dispatch_smoke"

SUBJECT = "grx015_gpu_culling_real_d3d12_dispatch_smoke"

WORD_STRIDE = 4  # uint / float lane stride (transforms, commands, visibility)
TRANSFORM_STRIDE_FLOATS = 12
COMMAND_STRIDE_DWORDS = 5
INSTANCE_COUNT_DWORD_INDEX = 1

# The kernel is pure integer word math (count + bitmask): the GPU output must
# match the CPU reference EXACTLY (zero tolerance).
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
        "grx015_gpu_culling_math_parity", MATH_PARITY_SCRIPT
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
    """Return None when the descriptor layout matches the tracked GRX-015
    gpu_culling resource mapping, otherwise a human-readable mismatch reason."""
    resources = layout.get("resources")
    expected = [
        ("src_transforms", "t", 0, "structured_buffer"),
        ("dst_commands", "u", 0, "rwstructured_buffer"),
        ("dst_visibility", "u", 1, "rwstructured_buffer"),
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
    if layout.get("root_constants") != 36:
        return "root_constants != 36"
    mapping = layout.get("grx015_mapping")
    if not isinstance(mapping, dict):
        return "missing grx015_mapping"
    if mapping.get("root_constant_bytes") != 144 or mapping.get("root_constant_dwords") != 36:
        return "root constant block is not 144 bytes / 36 dwords"
    if mapping.get("requires_64bit_integer_shader_capability") is not False:
        return "grx015_mapping must record requires_64bit_integer_shader_capability=false"
    if mapping.get("command_stride_dwords") != COMMAND_STRIDE_DWORDS:
        return "grx015_mapping command_stride_dwords is not 5"
    if mapping.get("instance_count_dword_index") != INSTANCE_COUNT_DWORD_INDEX:
        return "grx015_mapping instance_count_dword_index is not 1"
    if mapping.get("transform_stride_floats_in_scope") != TRANSFORM_STRIDE_FLOATS:
        return "grx015_mapping transform_stride_floats_in_scope is not 12"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx015-d3d12-dispatch-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx015-d3d12-dispatch-smoke] SKIP {msg}(降级 SKIP,退出 0)")
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
    print(f"[grx015-d3d12-dispatch-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


# ---------------------------------------------------------------------------
# Params-file builder (Python owns the exact bytes; the harness uploads them).
#
# Params binary format (little-endian):
#   uint32 transforms_bytes
#   uint32 commands_bytes         (the seeded initial command words)
#   uint32 visibility_bytes       (harness zero-uploads this many bytes)
#   uint8  b0[144]                (the 36-dword b0 mirror)
#   uint8  transforms[transforms_bytes]
#   uint8  commands_initial[commands_bytes]
#
# dst_commands is seeded with the tracked initial command words (count dwords
# start at 0); dst_visibility is explicitly zero-uploaded by the harness.
# ---------------------------------------------------------------------------
def build_case_payload(parity, case: dict) -> dict:
    n = case["instance_count"]
    consts = {
        "instance_count": n,
        "motion_vectors_current_offset": 0,
        "transform_stride_floats": TRANSFORM_STRIDE_FLOATS,
        "surface_count": case["surface_count"],
        "command_stride_dwords": COMMAND_STRIDE_DWORDS,
        "instance_count_dword_index": INSTANCE_COUNT_DWORD_INDEX,
        "planes": case["planes"],
    }
    instances = parity.build_instances(case["case_id"], n)
    transform_bytes, lanes = parity.pack_transforms(instances)
    result = parity.gpu_culling_reference(consts, lanes, instances)
    initial_commands = parity.initial_command_words(case["surface_count"])
    initial_command_bytes = struct.pack(f"<{len(initial_commands)}I", *initial_commands)
    b0 = parity.build_b0(consts)
    if len(b0) != 144:
        raise ValueError(f"build_b0 returned {len(b0)} bytes, expected 144")
    visibility_words = (n + 31) // 32
    visibility_bytes = visibility_words * WORD_STRIDE
    params = struct.pack("<III", len(transform_bytes), len(initial_command_bytes), visibility_bytes)
    params += b0 + transform_bytes + initial_command_bytes
    return {
        "consts": consts,
        "expected_commands": list(result["expected_command_words"]),
        "expected_bitmask": list(result["bitmask_words"]),
        "visible_count": result["visible_count"],
        "commands_bytes": len(initial_command_bytes),
        "visibility_bytes": visibility_bytes,
        "params": params,
    }


def compare_outputs(payload: dict, out_bin: Path) -> dict:
    raw = out_bin.read_bytes()
    commands_bytes = payload["commands_bytes"]
    visibility_bytes = payload["visibility_bytes"]
    if len(raw) != commands_bytes + visibility_bytes:
        return {"match": False, "reason": f"output size {len(raw)} != {commands_bytes + visibility_bytes}"}
    cmd_words = struct.unpack(f"<{commands_bytes // 4}I", raw[:commands_bytes])
    vis_words = struct.unpack(f"<{visibility_bytes // 4}I", raw[commands_bytes:])
    exp_cmd = payload["expected_commands"]
    exp_vis = payload["expected_bitmask"]
    cmd_mismatch = sum(1 for a, b in zip(exp_cmd, cmd_words) if a != b) + abs(len(exp_cmd) - len(cmd_words))
    vis_mismatch = sum(1 for a, b in zip(exp_vis, vis_words) if a != b) + abs(len(exp_vis) - len(vis_words))
    worst = None
    for idx, (a, b) in enumerate(zip(exp_cmd, cmd_words)):
        if a != b:
            worst = {"buffer": "dst_commands", "index": idx, "observed_hex": f"0x{b:08X}", "reference_hex": f"0x{a:08X}"}
            break
    if worst is None:
        for idx, (a, b) in enumerate(zip(exp_vis, vis_words)):
            if a != b:
                worst = {"buffer": "dst_visibility", "index": idx, "observed_hex": f"0x{b:08X}", "reference_hex": f"0x{a:08X}"}
                break
    return {
        "match": cmd_mismatch == 0 and vis_mismatch == 0,
        "mismatched_command_words": cmd_mismatch,
        "mismatched_visibility_words": vis_mismatch,
        "total_command_words": len(exp_cmd),
        "total_visibility_words": len(exp_vis),
        "value_tolerance": VALUE_TOLERANCE,
        "worst": worst,
    }


# ---------------------------------------------------------------------------
# Real D3D12 gpu_culling compute-dispatch harness (C++/MSVC), on demand.
# Root param 0 = 36-dword (144-byte) b0; root param 1 = descriptor table
# [ SRV t0 src_transforms, UAV u0 dst_commands, UAV u1 dst_visibility ].
# dst_commands is seeded from the params (count dwords start at 0); dst_visibility
# is zero-uploaded. Dispatch = (ceil(instance_count / 64), 1, 1) with
# instance_count read from b0 dword 24. The harness writes dst_commands readback
# followed by dst_visibility readback to <out_bin>.
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

static const UINT WORD_STRIDE = 4u;
static const UINT INSTANCE_COUNT_DWORD = 24u; // b0 dword index of instance_count

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

static void make_structured_srv(ID3D12Device* d, ID3D12Resource* r, UINT bytes,
                                D3D12_CPU_DESCRIPTOR_HANDLE h) {
    D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
    srv.Format = DXGI_FORMAT_UNKNOWN;
    srv.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
    srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    srv.Buffer.FirstElement = 0;
    srv.Buffer.NumElements = std::max<UINT>(bytes / WORD_STRIDE, 1u);
    srv.Buffer.StructureByteStride = WORD_STRIDE;
    srv.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
    d->CreateShaderResourceView(r, &srv, h);
}
static void make_structured_uav(ID3D12Device* d, ID3D12Resource* r, UINT bytes,
                                D3D12_CPU_DESCRIPTOR_HANDLE h) {
    D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
    uav.Format = DXGI_FORMAT_UNKNOWN;
    uav.ViewDimension = D3D12_UAV_DIMENSION_BUFFER;
    uav.Buffer.FirstElement = 0;
    uav.Buffer.NumElements = std::max<UINT>(bytes / WORD_STRIDE, 1u);
    uav.Buffer.StructureByteStride = WORD_STRIDE;
    uav.Buffer.CounterOffsetInBytes = 0;
    uav.Buffer.Flags = D3D12_BUFFER_UAV_FLAG_NONE;
    d->CreateUnorderedAccessView(r, nullptr, &uav, h);
}

int wmain(int argc, wchar_t** argv) {
    if (argc < 5 || argc > 6) return fail_msg("usage: harness dxil rts0 params_bin out_bin [dxil.dll]");
    bool ok_dxil = false, ok_rts0 = false, ok_params = false;
    auto dxil = read_file(argv[1], &ok_dxil);
    const auto rts0 = read_file(argv[2], &ok_rts0);
    const auto params = read_file(argv[3], &ok_params);
    if (!ok_dxil || dxil.empty()) return fail_msg("read dxil");
    if (!ok_rts0 || rts0.empty()) return fail_msg("read rts0");
    if (!ok_params || params.size() < 12 + 144) return fail_msg("read params");
    const wchar_t* out_bin = argv[4];
    const wchar_t* dxil_dll = (argc >= 6) ? argv[5] : nullptr;

    UINT transforms_bytes = 0, commands_bytes = 0, visibility_bytes = 0;
    std::memcpy(&transforms_bytes, params.data() + 0, 4);
    std::memcpy(&commands_bytes, params.data() + 4, 4);
    std::memcpy(&visibility_bytes, params.data() + 8, 4);
    const uint8_t* b0 = params.data() + 12;
    const uint8_t* transforms_data = params.data() + 12 + 144;
    const uint8_t* commands_data = transforms_data + transforms_bytes;
    if (params.size() != (size_t)(12 + 144 + transforms_bytes + commands_bytes))
        return fail_msg("params size mismatch");
    if (transforms_bytes == 0 || transforms_bytes % WORD_STRIDE != 0) return fail_msg("transforms_bytes");
    if (commands_bytes == 0 || commands_bytes % WORD_STRIDE != 0) return fail_msg("commands_bytes");
    if (visibility_bytes == 0 || visibility_bytes % WORD_STRIDE != 0) return fail_msg("visibility_bytes");

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

    std::vector<uint8_t> vis_zeros((size_t)visibility_bytes, 0u);
    ComPtr<ID3D12Resource> tf_buf, tf_up, cmd_buf, cmd_up, vis_buf, vis_up;
    if (!make_buffer(device.Get(), cmd.Get(), transforms_bytes, D3D12_RESOURCE_FLAG_NONE,
                     D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, transforms_data, tf_buf, tf_up,
                     "src_transforms"))
        return 1;
    if (!make_buffer(device.Get(), cmd.Get(), commands_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
                     D3D12_RESOURCE_STATE_UNORDERED_ACCESS, commands_data, cmd_buf, cmd_up,
                     "dst_commands"))
        return 1;
    if (!make_buffer(device.Get(), cmd.Get(), visibility_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
                     D3D12_RESOURCE_STATE_UNORDERED_ACCESS, vis_zeros.data(), vis_buf, vis_up,
                     "dst_visibility"))
        return 1;

    D3D12_DESCRIPTOR_HEAP_DESC hd = {};
    hd.NumDescriptors = 3;
    hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap;
    if (FAILED(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap))))
        return fail_msg("CreateDescriptorHeap(cbv_srv_uav)");
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    D3D12_CPU_DESCRIPTOR_HANDLE cpu = heap->GetCPUDescriptorHandleForHeapStart();
    make_structured_srv(device.Get(), tf_buf.Get(), transforms_bytes, cpu);
    D3D12_CPU_DESCRIPTOR_HANDLE h1 = cpu; h1.ptr += (SIZE_T)inc;
    make_structured_uav(device.Get(), cmd_buf.Get(), commands_bytes, h1);
    D3D12_CPU_DESCRIPTOR_HANDLE h2 = cpu; h2.ptr += (SIZE_T)inc * 2;
    make_structured_uav(device.Get(), vis_buf.Get(), visibility_bytes, h2);

    cmd->SetComputeRootSignature(root.Get());
    ID3D12DescriptorHeap* heaps[] = {heap.Get()};
    cmd->SetDescriptorHeaps(1, heaps);
    uint32_t rc[36];
    std::memcpy(rc, b0, 144);
    cmd->SetComputeRoot32BitConstants(0, 36, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetPipelineState(pso.Get());
    const uint32_t instance_count = rc[INSTANCE_COUNT_DWORD];
    const UINT gx = std::max<UINT>((instance_count + 63u) / 64u, 1u);
    cmd->Dispatch(gx, 1, 1);

    // Readback buffers for both UAVs.
    auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);
    ComPtr<ID3D12Resource> rb_cmd, rb_vis;
    {
        auto d1 = buffer_desc(commands_bytes, D3D12_RESOURCE_FLAG_NONE);
        if (FAILED(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &d1,
                   D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&rb_cmd))))
            return fail_msg("CreateCommittedResource(readback commands)");
        auto d2 = buffer_desc(visibility_bytes, D3D12_RESOURCE_FLAG_NONE);
        if (FAILED(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &d2,
                   D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&rb_vis))))
            return fail_msg("CreateCommittedResource(readback visibility)");
    }
    D3D12_RESOURCE_BARRIER tb[2] = {};
    tb[0].Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    tb[0].Transition.pResource = cmd_buf.Get();
    tb[0].Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    tb[0].Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
    tb[0].Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    tb[1] = tb[0];
    tb[1].Transition.pResource = vis_buf.Get();
    cmd->ResourceBarrier(2, tb);
    cmd->CopyBufferRegion(rb_cmd.Get(), 0, cmd_buf.Get(), 0, commands_bytes);
    cmd->CopyBufferRegion(rb_vis.Get(), 0, vis_buf.Get(), 0, visibility_bytes);
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

    std::vector<uint8_t> out_bytes;
    out_bytes.resize((size_t)commands_bytes + visibility_bytes);
    {
        uint8_t* mapped = nullptr;
        D3D12_RANGE range = {0, (SIZE_T)commands_bytes};
        if (FAILED(rb_cmd->Map(0, &range, reinterpret_cast<void**>(&mapped)))) return fail_msg("Map readback commands");
        std::memcpy(out_bytes.data(), mapped, commands_bytes);
        rb_cmd->Unmap(0, nullptr);
    }
    {
        uint8_t* mapped = nullptr;
        D3D12_RANGE range = {0, (SIZE_T)visibility_bytes};
        if (FAILED(rb_vis->Map(0, &range, reinterpret_cast<void**>(&mapped)))) return fail_msg("Map readback visibility");
        std::memcpy(out_bytes.data() + commands_bytes, mapped, visibility_bytes);
        rb_vis->Unmap(0, nullptr);
    }
    std::ofstream of(out_bin, std::ios::binary);
    if (!of) return fail_msg("open out_bin");
    of.write(reinterpret_cast<const char*>(out_bytes.data()), (std::streamsize)out_bytes.size());
    of.close();
    if (!of) return fail_msg("write out_bin");

    uint32_t checksum = 2166136261u;
    for (uint8_t b : out_bytes) { checksum ^= b; checksum *= 16777619u; }
    std::printf("RXGD_DISPATCH: ok adapter=\"%s\" dispatch=%u,1,1 fence=%llu instance_count=%u "
                "command_bytes=%u visibility_bytes=%u checksum=0x%08x\n",
                narrow(chosen_desc.Description).c_str(), gx, (unsigned long long)fence_done,
                instance_count, commands_bytes, visibility_bytes, checksum);
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
            for token in ("dispatch=", "fence=", "instance_count=", "checksum="):
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
        return fail("cannot read gpu_culling_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "gpu_culling",
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
            "GRX-015 standalone real D3D12 gpu_culling dispatch smoke evidence "
            "only (1 SRV + 2 UAV structured buffers). A success flips "
            "real_d3d12_dispatch_recorded/cpu_reference_match true but keeps "
            "runtime_state=fallback_only and real_gpu_pass=false; it is not a "
            "Godot runtime pass, visual, perf, or measured-telemetry claim."
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
    if parity is None or not hasattr(parity, "gpu_culling_reference") or not hasattr(parity, "parity_cases"):
        return fail(
            "cannot import the tracked generate_math_parity_evidence.py reference "
            "implementation (gpu_culling_reference / parity_cases) for the CPU cross-check"
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
    cpp = WORK / "gpu_culling_dispatch_harness.cpp"
    exe = WORK / "gpu_culling_dispatch_harness.exe"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")

    built, build_log = compile_harness(vcvars, cpp, exe, include_dir)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 dispatch harness 失败(可能缺 Windows SDK D3D12 头/库)",
                    extra={"build_log_tail": build_log})

    device_info: dict = {}
    case_results: list[dict] = []
    all_match = True
    for case in parity.parity_cases():
        payload = build_case_payload(parity, case)
        params_bin = WORK / f"params_{case['case_id']}.bin"
        out_bin = WORK / f"out_{case['case_id']}.bin"
        params_bin.write_bytes(payload["params"])
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
            return fail(f"real D3D12 gpu_culling dispatch smoke failed for case {case['case_id']}",
                        extra={"device": device_info, "exit_code": p.returncode, "stdout": output})

        comparison = compare_outputs(payload, out_bin)
        if not comparison.get("match"):
            all_match = False
        consts = payload["consts"]
        case_results.append({
            "case_id": case["case_id"],
            "instance_count": consts["instance_count"],
            "surface_count": consts["surface_count"],
            "visible_count": payload["visible_count"],
            "dispatch": parsed.get("dispatch"),
            "fence_completed_value": parsed.get("fence"),
            "readback_checksum": parsed.get("checksum"),
            "comparison": comparison,
        })

    if not all_match:
        return fail(
            "GPU-observed gpu_culling command/visibility words did not match the tracked reference exactly",
            extra={"device": device_info, "cases": case_results},
        )

    cpu_reference = {
        "reference_impl": (
            "spike/godot-rurix/passes/gpu_culling/generate_math_parity_evidence.py "
            "gpu_culling_reference (imported; every command + visibility u32 word compared exactly)"
        ),
        "value_tolerance": VALUE_TOLERANCE,
        "cases": case_results,
    }
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
                "one_srv_two_uav_bound_from_layout": True,
                "dst_commands_seeded_dst_visibility_zeroed": True,
                "dispatch_executed": True,
                "fence_completed": True,
                "both_uav_readback": True,
                "all_output_words_match_cpu_reference_exactly": True,
            },
        },
    )
    print(f"[grx015-d3d12-dispatch-smoke] PASS measured real D3D12 dispatch over "
          f"{len(case_results)} cases; adapter={device_info.get('adapter')} tolerance=0 (exact)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
