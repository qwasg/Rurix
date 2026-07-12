// rxgd_luminance_record.cpp — GRX-009 bridge D3D12 dispatch recording shim.
//
// Compiled only under the rurix-godot `d3d12-recording-shim` feature (see
// build.rs). It records REAL D3D12 compute dispatches on a caller-provided
// D3D12 device / command queue and REAL ID3D12Resource* handles, using tracked
// offline DXIL containers + RTS0 root signatures (bytes are passed in by the
// Rust bridge, which first verified they hash to the offline compile evidence
// digests).
//
// This shim NEVER creates a device, NEVER accepts fake/null handles, and NEVER
// fakes a dispatch: any failure returns a negative status and the bridge falls
// back. It does not enable the Godot runtime luminance path.
//
// ── Wave 2 execution model v2 (shim ABI 2) ────────────────────────────────
// The v1 shim re-created a root signature, PSO, command allocator, descriptor
// heap, fence and readback buffer on EVERY dispatch and always blocked on an
// INFINITE fence wait. v2 introduces a per-(device,queue) `ShimSession` that:
//   * caches root signature + PSO once per kernel identity (FNV-1a of the DXIL
//     container ^ FNV-1a of the RTS0 bytes),
//   * keeps a command-allocator ring (>= 3 slots) recycled by fence value,
//   * keeps one shader-visible CBV_SRV_UAV descriptor heap sub-allocated by a
//     rolling cursor,
//   * keeps one shared fence + event,
//   * records per-pass recorded/fallback counters and prints ONE machine-
//     readable `RXGD_SUMMARY pass=<id> recorded=<n> fallback=<n>` line per pass
//     when the session is closed (rxgd_luminance_record_shim_session_close,
//     driven from the Rust rxgd_destroy_session under the feature).
//
// Two record entry points share a single `ShimSession::record_levels` core:
//   * rxgd_luminance_record_dispatch(...) — the historical single-level
//     2-resource (SRV t0 src, UAV u0 dst) path. Always runs in TEST-ONLY
//     READBACK mode (readback=true): it dispatches, transitions the dst UAV to
//     COPY_SOURCE, copies to a per-call readback buffer, WAITS on the fence,
//     checksums the readback, and prints the per-frame `RXGD_BRIDGE_REC:`
//     marker that ci/grx009_luminance_bridge_recording_smoke.py parses. The 4c
//     /4d smoke semantics are unchanged.
//   * rxgd_luminance_record_levels(...) — the multi-kernel / multi-resource /
//     multi-level pyramid path. It records K dispatches (reduce chain + final
//     WRITE_LUMINANCE) with inter-level UAV/state barriers in ONE command list
//     and ONE submit. In PRODUCTION mode (readback=false) it records the fence
//     value into the allocator ring and DOES NOT block (allocators are recycled
//     by checking fence completion before reuse); no per-frame marker is
//     printed. In readback mode it also reads back the final level. This entry
//     is defined for the later enablement smoke slices (patch side); no
//     real-GPU test in this cargo-only slice drives it.
//
// ── ONE-FRAME LATENCY (honest, documented) ────────────────────────────────
// When the Godot runtime hook records these dispatches from within a frame,
// Godot has not yet submitted that frame's own rendering to the queue, so a
// self-queue dispatch that reads Godot's internal_texture reads the PREVIOUS
// frame's content. The luminance pass uses time-domain EMA feedback, so a 1
// frame delay is defensible, but it must be recorded as such (see
// hook_contract_v2.md and math_parity_evidence.json semantics). This shim does
// not hide it: it records exactly what it is handed.
//
// Layout (from the tracked descriptor layouts):
//   root param 0 = 7-dword (28-byte) b0 root constants
//                  (source_width/source_height as i64 + 3 f32 scalars)
//   root param 1 = descriptor table:
//                    reduce kernel : [ SRV t0 (src), UAV u0 (dst) ]
//                    write kernel  : [ SRV t0 (src), SRV t1 (prev), UAV u0 (dst) ]

#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <map>
#include <mutex>
#include <string>
#include <utility>
#include <vector>

#ifdef RXGD_HAVE_DXCAPI
#include <dxcapi.h>
#endif

using Microsoft::WRL::ComPtr;

// Shim <-> Rust ABI version (kept in sync with the Rust d3d12_shim module).
// Bumped 1 -> 2 for the Wave 2 v2 execution model (session cache + rings +
// multi-level record entry + summary + explicit session close).
// Bumped 2 -> 3 for the Wave 4 production-dispatch split: the single-dispatch
// record entries (2-resource / taa / particles) now take an explicit `readback`
// selector so the shipping/bench real-pass path can run with ZERO per-dispatch
// readback / fence-wait / checksum / stdout marker (production mode), while the
// test/recording arms opt in to the instrumented readback + RXGD_BRIDGE_REC
// marker by passing readback=1.
static const uint32_t kShimAbiVersion = 3u;

// Command-allocator ring depth. >= 3 so several frames of dispatches can be in
// flight while older allocators are recycled by fence completion.
static const size_t kAllocatorRingDepth = 3u;
// Shader-visible descriptor heap capacity (rolling sub-allocation). Generous so
// a full multi-level pyramid submission (per level: 2-3 descriptors) fits many
// times before the cursor wraps.
static const UINT kDescriptorHeapCapacity = 1024u;

extern "C" struct RxgdRecordResult {
    uint64_t fence_completed_value;
    uint32_t dispatch_x;
    uint32_t dispatch_y;
    uint32_t dispatch_z;
    uint32_t dst_width;
    uint32_t dst_height;
    uint32_t readback_checksum;
    float dst_first_value;
    int32_t dxil_signed;      // 1 = all in-memory DXIL kernels were signed
    char error_detail[256];
};

// One bound resource for a multi-level job.
extern "C" struct RxgdShimResource {
    void* resource;           // ID3D12Resource*
    uint32_t reserved0;       // padding / future flags (must be 0)
    uint32_t reserved1;
};

// One kernel's tracked bytes for a multi-level job.
extern "C" struct RxgdShimKernel {
    const uint8_t* dxil;
    size_t dxil_len;
    const uint8_t* rts0;
    size_t rts0_len;
    uint32_t binding_count;   // 2 = reduce (SRV+UAV), 3 = write (SRV+SRV+UAV)
    uint32_t reserved0;
};

// One dispatch level in a multi-level sequence.
extern "C" struct RxgdShimLevel {
    uint32_t kernel_index;    // index into the kernels[] array
    uint32_t srv_index;       // index into resources[] for SRV t0 (source)
    uint32_t uav_index;       // index into resources[] for UAV u0 (dest)
    uint32_t prev_index;      // index into resources[] for SRV t1 (prev), write only
    uint32_t dispatch_x;
    uint32_t dispatch_y;
    uint32_t dispatch_z;
    uint32_t dst_width;       // dst UAV texel extent (for readback footprint)
    uint32_t dst_height;
    uint8_t push_constants[28];
};

static void set_detail(RxgdRecordResult* out, const char* what, HRESULT hr) {
    if (!out) return;
    std::snprintf(out->error_detail, sizeof(out->error_detail),
                  "%s hr=0x%08lx", what ? what : "", (unsigned long)hr);
}
static void set_detail_msg(RxgdRecordResult* out, const char* what) {
    if (!out) return;
    std::snprintf(out->error_detail, sizeof(out->error_detail), "%s",
                  what ? what : "");
}

static uint64_t fnv1a64(const uint8_t* data, size_t len) {
    uint64_t h = 1469598103934665603ull;  // FNV offset basis
    for (size_t i = 0; i < len; ++i) {
        h ^= data[i];
        h *= 1099511628211ull;  // FNV prime
    }
    return h;
}

static D3D12_HEAP_PROPERTIES heap_props(D3D12_HEAP_TYPE type) {
    D3D12_HEAP_PROPERTIES hp = {};
    hp.Type = type;
    hp.CreationNodeMask = 1;
    hp.VisibleNodeMask = 1;
    return hp;
}
// Resolve a (possibly typeless) resource format to a concrete typed format that
// is view-compatible with the resource's format family. Godot creates its HDR
// color / luminance textures with *typeless* formats (e.g. R16G16B16A16_TYPELESS
// for the scene color, R32_TYPELESS for the reduction target). Creating a view
// whose format is not in the resource's typeless family (the old shim hardcoded
// R32_FLOAT for both) is invalid and on some drivers removes the device
// (DXGI_ERROR_DEVICE_HUNG). This maps the common typeless families to a typed
// member and passes already-typed formats through unchanged.
static DXGI_FORMAT typed_view_format(DXGI_FORMAT resource_format) {
    switch (resource_format) {
        case DXGI_FORMAT_R32G32B32A32_TYPELESS:
            return DXGI_FORMAT_R32G32B32A32_FLOAT;
        case DXGI_FORMAT_R32G32B32_TYPELESS:
            return DXGI_FORMAT_R32G32B32_FLOAT;
        case DXGI_FORMAT_R16G16B16A16_TYPELESS:
            return DXGI_FORMAT_R16G16B16A16_FLOAT;
        case DXGI_FORMAT_R32G32_TYPELESS:
            return DXGI_FORMAT_R32G32_FLOAT;
        case DXGI_FORMAT_R10G10B10A2_TYPELESS:
            return DXGI_FORMAT_R10G10B10A2_UNORM;
        case DXGI_FORMAT_R8G8B8A8_TYPELESS:
            return DXGI_FORMAT_R8G8B8A8_UNORM;
        case DXGI_FORMAT_R16G16_TYPELESS:
            return DXGI_FORMAT_R16G16_FLOAT;
        case DXGI_FORMAT_R32_TYPELESS:
            return DXGI_FORMAT_R32_FLOAT;
        // GRX-012: the Godot depth buffer bound by the TAA resolve (t1,
        // sampler2D depth_buffer) is a combined depth-stencil resource created
        // with a typeless family (R32G8X24 = 32-bit float depth + 8-bit
        // stencil, or R24G8 = 24-bit unorm depth + 8-bit stencil). A compute
        // SRV reads the DEPTH PLANE through the corresponding *_X8X24 /
        // X8_TYPELESS depth-read format; leaving these unmapped left the view
        // format typeless (an invalid, device-removing D3D12 call), so the old
        // shim fail-closed with "unmapped typeless format" on every TAA frame.
        case DXGI_FORMAT_R32G8X24_TYPELESS:
            return DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS;
        case DXGI_FORMAT_R24G8_TYPELESS:
            return DXGI_FORMAT_R24_UNORM_X8_TYPELESS;
        // GRX-011: the Godot SSAO deinterleaved AO buffers are created with the
        // R8G8 typeless family (RD::DATA_FORMAT_R8G8_UNORM lowers to an
        // R8G8_TYPELESS ID3D12Resource); the old shim left this UNMAPPED, so it
        // created a view with a *typeless* format — an invalid D3D12 call that
        // removes the device with DXGI_ERROR_INVALID_CALL. Map it (and the other
        // narrow families) to their UNORM/FLOAT typed member.
        case DXGI_FORMAT_R8G8_TYPELESS:
            return DXGI_FORMAT_R8G8_UNORM;
        case DXGI_FORMAT_R16_TYPELESS:
            return DXGI_FORMAT_R16_FLOAT;
        case DXGI_FORMAT_R8_TYPELESS:
            return DXGI_FORMAT_R8_UNORM;
        case DXGI_FORMAT_B8G8R8A8_TYPELESS:
            return DXGI_FORMAT_B8G8R8A8_UNORM;
        case DXGI_FORMAT_B8G8R8X8_TYPELESS:
            return DXGI_FORMAT_B8G8R8X8_UNORM;
        case DXGI_FORMAT_UNKNOWN:
            return DXGI_FORMAT_R32_FLOAT;
        default:
            // Already a typed format: use it as-is so the view matches the
            // resource exactly.
            return resource_format;
    }
}

// Fail-closed guard: creating an SRV/UAV with a *typeless* format is an invalid
// D3D12 call that removes the device. If a resource carries a typeless family
// this shim does not yet map to a typed member, `typed_view_format` returns the
// typeless format unchanged; the caller must detect that and fall back cleanly
// (return an error) instead of issuing the device-removing invalid call.
static bool format_is_typeless(DXGI_FORMAT f) {
    switch (f) {
        case DXGI_FORMAT_R32G32B32A32_TYPELESS:
        case DXGI_FORMAT_R32G32B32_TYPELESS:
        case DXGI_FORMAT_R16G16B16A16_TYPELESS:
        case DXGI_FORMAT_R32G32_TYPELESS:
        case DXGI_FORMAT_R32G8X24_TYPELESS:
        case DXGI_FORMAT_R10G10B10A2_TYPELESS:
        case DXGI_FORMAT_R8G8B8A8_TYPELESS:
        case DXGI_FORMAT_R16G16_TYPELESS:
        case DXGI_FORMAT_R32_TYPELESS:
        case DXGI_FORMAT_R24G8_TYPELESS:
        case DXGI_FORMAT_R8G8_TYPELESS:
        case DXGI_FORMAT_R16_TYPELESS:
        case DXGI_FORMAT_R8_TYPELESS:
        case DXGI_FORMAT_BC1_TYPELESS:
        case DXGI_FORMAT_BC2_TYPELESS:
        case DXGI_FORMAT_BC3_TYPELESS:
        case DXGI_FORMAT_BC4_TYPELESS:
        case DXGI_FORMAT_BC5_TYPELESS:
        case DXGI_FORMAT_B8G8R8A8_TYPELESS:
        case DXGI_FORMAT_B8G8R8X8_TYPELESS:
        case DXGI_FORMAT_BC6H_TYPELESS:
        case DXGI_FORMAT_BC7_TYPELESS:
            return true;
        default:
            return false;
    }
}

// Bytes per pixel of a typed (non-block, non-planar) DXGI format. Used to size
// the test-only readback iteration so it never reads past the actual per-pixel
// footprint (the old shim assumed a fixed 4-byte RGBA/R32 stride, which
// overran a 2-byte R8G8 readback buffer on the SSAO path). Returns 0 for
// formats this helper does not size, so the caller can clamp defensively.
static UINT dxgi_format_bytes(DXGI_FORMAT f) {
    switch (f) {
        case DXGI_FORMAT_R32G32B32A32_FLOAT:
        case DXGI_FORMAT_R32G32B32A32_UINT:
        case DXGI_FORMAT_R32G32B32A32_SINT:
            return 16;
        case DXGI_FORMAT_R32G32B32_FLOAT:
            return 12;
        case DXGI_FORMAT_R16G16B16A16_FLOAT:
        case DXGI_FORMAT_R16G16B16A16_UNORM:
        case DXGI_FORMAT_R16G16B16A16_UINT:
        case DXGI_FORMAT_R16G16B16A16_SNORM:
        case DXGI_FORMAT_R16G16B16A16_SINT:
        case DXGI_FORMAT_R32G32_FLOAT:
        case DXGI_FORMAT_R32G32_UINT:
            return 8;
        case DXGI_FORMAT_R8G8B8A8_UNORM:
        case DXGI_FORMAT_R8G8B8A8_UNORM_SRGB:
        case DXGI_FORMAT_R8G8B8A8_UINT:
        case DXGI_FORMAT_B8G8R8A8_UNORM:
        case DXGI_FORMAT_B8G8R8X8_UNORM:
        case DXGI_FORMAT_R10G10B10A2_UNORM:
        case DXGI_FORMAT_R11G11B10_FLOAT:
        case DXGI_FORMAT_R16G16_FLOAT:
        case DXGI_FORMAT_R16G16_UNORM:
        case DXGI_FORMAT_R32_FLOAT:
        case DXGI_FORMAT_R32_UINT:
        case DXGI_FORMAT_R32_SINT:
        case DXGI_FORMAT_D32_FLOAT:
            return 4;
        case DXGI_FORMAT_R8G8_UNORM:
        case DXGI_FORMAT_R8G8_UINT:
        case DXGI_FORMAT_R8G8_SNORM:
        case DXGI_FORMAT_R16_FLOAT:
        case DXGI_FORMAT_R16_UNORM:
        case DXGI_FORMAT_R16_UINT:
        case DXGI_FORMAT_D16_UNORM:
            return 2;
        case DXGI_FORMAT_R8_UNORM:
        case DXGI_FORMAT_R8_UINT:
        case DXGI_FORMAT_R8_SNORM:
        case DXGI_FORMAT_A8_UNORM:
            return 1;
        default:
            return 0;
    }
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

#ifdef RXGD_HAVE_DXCAPI
// Minimal in-memory IDxcBlob so the DXIL validator can sign our container bytes
// in place (DxcValidatorFlags_InPlaceEdit writes the validation hash directly
// into the buffer we own). Mirrors the segment 4c smoke's MemBlob.
struct MemBlob : public IDxcBlob {
    LONG m_ref;
    void* m_ptr;
    SIZE_T m_size;
    MemBlob(void* p, SIZE_T s) : m_ref(1), m_ptr(p), m_size(s) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == __uuidof(IUnknown) || riid == __uuidof(IDxcBlob)) {
            *ppv = static_cast<IDxcBlob*>(this);
            AddRef();
            return S_OK;
        }
        *ppv = nullptr;
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return (ULONG)InterlockedIncrement(&m_ref); }
    ULONG STDMETHODCALLTYPE Release() override { return (ULONG)InterlockedDecrement(&m_ref); }
    LPVOID STDMETHODCALLTYPE GetBufferPointer() override { return m_ptr; }
    SIZE_T STDMETHODCALLTYPE GetBufferSize() override { return m_size; }
};

// Sign the DXIL container in place with the DXIL validator (dxil.dll). Loads
// dxil.dll from RURIX_DXC_DIR / RURIX_DXC_NEW_DIR (or the system search path).
// Signing does not change shader semantics; it appends the validation hash so
// the same container can create a compute PSO on a normal device. The tracked
// artifact file on disk is never touched — only the caller-owned memory copy.
static bool sign_dxil_in_place(std::vector<uint8_t>& dxil) {
    HMODULE lib = nullptr;
    for (const wchar_t* key : {L"RURIX_DXC_DIR", L"RURIX_DXC_NEW_DIR"}) {
        wchar_t buf[1024];
        DWORD n = GetEnvironmentVariableW(key, buf, 1024);
        if (n == 0 || n >= 1024) continue;
        std::wstring path(buf, n);
        if (!path.empty() && path.back() != L'\\' && path.back() != L'/') path.push_back(L'\\');
        path += L"dxil.dll";
        lib = LoadLibraryW(path.c_str());
        if (lib) break;
    }
    if (!lib) lib = LoadLibraryW(L"dxil.dll");
    if (!lib) return false;
    auto create = reinterpret_cast<DxcCreateInstanceProc>(GetProcAddress(lib, "DxcCreateInstance"));
    if (!create) return false;
    IDxcValidator* validator = nullptr;
    if (FAILED(create(CLSID_DxcValidator, __uuidof(IDxcValidator),
                      reinterpret_cast<void**>(&validator))) || !validator) {
        return false;
    }
    MemBlob blob(dxil.data(), dxil.size());
    IDxcOperationResult* result = nullptr;
    HRESULT hr = validator->Validate(&blob, DxcValidatorFlags_InPlaceEdit, &result);
    bool ok = false;
    if (SUCCEEDED(hr) && result) {
        HRESULT status = E_FAIL;
        result->GetStatus(&status);
        ok = SUCCEEDED(status);
    }
    if (result) result->Release();
    validator->Release();
    return ok;
}
#else
static bool sign_dxil_in_place(std::vector<uint8_t>&) {
    // dxcapi.h was not available at build time (no signed DXC pin located).
    // Recording can only succeed on a Developer-Mode device with experimental
    // shader models; otherwise CreateComputePipelineState will fail and the
    // bridge falls back. We never fake success.
    return false;
}
#endif

// ── Wave 4 engagement counter file (production-safe telemetry) ─────────────
// The production real-pass path prints NO per-dispatch stdout, so the bench
// runner can no longer scrape per-frame markers for pass engagement, and the
// RXGD_SUMMARY stdout line only appears when the session is closed cleanly
// (which a Godot force-quit can skip). To make engagement reporting reliable
// the session mirrors its per-pass recorded/fallback counters to a JSON file
// both periodically (every kEngagementFlushInterval notes) and at session
// close. The path comes from the RXGD_ENGAGEMENT_OUTPUT environment variable
// (injected by the runner / harness). Writing is best-effort: any failure is
// swallowed so it never perturbs rendering. Format:
//   {"<pass_id>": {"recorded": n, "fallback": m}, ...}
// with numeric pass_id keys (the runner maps them to pass names).
static const uint64_t kEngagementFlushInterval = 256u;

// Resolve the engagement output path from the environment exactly once. Returns
// an empty string when RXGD_ENGAGEMENT_OUTPUT is unset (engagement file
// disabled — the historical stdout RXGD_SUMMARY path is unaffected).
static const std::string& engagement_output_path() {
    static const std::string path = []() -> std::string {
        char buf[1024];
        DWORD n = GetEnvironmentVariableA("RXGD_ENGAGEMENT_OUTPUT", buf,
                                          (DWORD)sizeof(buf));
        if (n == 0 || n >= sizeof(buf)) return std::string();
        return std::string(buf, n);
    }();
    return path;
}

// Atomically replace `path` with `content`: write a sibling .tmp file then
// MoveFileExA(REPLACE_EXISTING). Best-effort — every failure is ignored so a
// read-only directory / locked file never disturbs rendering.
static void atomic_write_engagement(const std::string& path, const std::string& content) {
    if (path.empty()) return;
    std::string tmp = path + ".tmp";
    HANDLE h = CreateFileA(tmp.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS,
                           FILE_ATTRIBUTE_NORMAL, nullptr);
    if (h == INVALID_HANDLE_VALUE) return;
    const char* data = content.data();
    size_t remaining = content.size();
    bool ok = true;
    while (remaining > 0) {
        DWORD chunk = (DWORD)std::min<size_t>(remaining, 1u << 20);
        DWORD written = 0;
        if (!WriteFile(h, data, chunk, &written, nullptr) || written == 0) {
            ok = false;
            break;
        }
        data += written;
        remaining -= written;
    }
    CloseHandle(h);
    if (!ok) {
        DeleteFileA(tmp.c_str());
        return;
    }
    if (!MoveFileExA(tmp.c_str(), path.c_str(),
                     MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        DeleteFileA(tmp.c_str());
    }
}

// ── Session-cached kernel / allocator ring / descriptor heap ───────────────

struct CachedKernel {
    ComPtr<ID3D12RootSignature> root;
    ComPtr<ID3D12PipelineState> pso;
    bool dxil_signed = false;
};

struct AllocatorSlot {
    ComPtr<ID3D12CommandAllocator> alloc;
    UINT64 fence_value = 0;  // fence value the last submit using this slot signals
};

struct PassCounters {
    uint64_t recorded = 0;
    uint64_t fallback = 0;
};

// GRX-020: one committed descriptor-heap sub-range and the fence value of the
// submit that consumes it. The reserve path tracks these so a wrapped
// reservation waits for any overlapping in-flight range to complete before
// reuse (segmented fence-value reclaim, mirroring the allocator ring), instead
// of the old blind `heap_cursor = 0` reset that could stomp descriptors still
// referenced by an unfinished submit.
struct DescriptorSegment {
    UINT begin;
    UINT end;             // exclusive
    UINT64 fence_value;   // submit value that will/does consume this range
};

struct ShimSession {
    ID3D12Device* device;         // borrowed (caller-owned, not AddRef'd)
    ID3D12CommandQueue* queue;    // borrowed (caller-owned, not AddRef'd)

    std::map<uint64_t, CachedKernel> kernels;  // key = fnv(dxil) ^ rotl(fnv(rts0))
    std::vector<AllocatorSlot> allocators;
    size_t alloc_cursor = 0;
    ComPtr<ID3D12GraphicsCommandList> cmd;
    ComPtr<ID3D12DescriptorHeap> heap;
    UINT descriptor_increment = 0;
    UINT heap_cursor = 0;
    // GRX-020: in-flight descriptor sub-ranges + wrap/wait telemetry.
    std::vector<DescriptorSegment> descriptor_ring;
    uint64_t descriptor_ring_wraps = 0;
    uint64_t descriptor_ring_waits = 0;
    ComPtr<ID3D12Fence> fence;
    UINT64 next_fence_value = 0;
    HANDLE fence_event = nullptr;
    bool initialized = false;
    std::map<uint32_t, PassCounters> counters;
    uint64_t note_count = 0;  // total note() calls, drives periodic engagement flush

    ShimSession(ID3D12Device* d, ID3D12CommandQueue* q) : device(d), queue(q) {}
    ~ShimSession() {
        if (fence_event) CloseHandle(fence_event);
    }

    // Lazily create the allocator ring, descriptor heap, shared fence + event
    // and the reusable command list. Kept out of the constructor so a session
    // that never records touches no D3D12 state.
    bool ensure_initialized(RxgdRecordResult* out) {
        if (initialized) return true;
        allocators.resize(kAllocatorRingDepth);
        for (auto& slot : allocators) {
            HRESULT hr = device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT,
                                                        IID_PPV_ARGS(&slot.alloc));
            if (FAILED(hr)) {
                set_detail(out, "CreateCommandAllocator(ring)", hr);
                return false;
            }
        }
        D3D12_DESCRIPTOR_HEAP_DESC hd = {};
        hd.NumDescriptors = kDescriptorHeapCapacity;
        hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
        hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
        HRESULT hr = device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap));
        if (FAILED(hr)) {
            set_detail(out, "CreateDescriptorHeap(session ring)", hr);
            return false;
        }
        descriptor_increment =
            device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
        hr = device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence));
        if (FAILED(hr)) {
            set_detail(out, "CreateFence(session)", hr);
            return false;
        }
        fence_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!fence_event) {
            set_detail_msg(out, "CreateEvent(session)");
            return false;
        }
        // One reusable command list, created closed (Reset per record).
        hr = device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT,
                                       allocators[0].alloc.Get(), nullptr,
                                       IID_PPV_ARGS(&cmd));
        if (FAILED(hr)) {
            set_detail(out, "CreateCommandList(session)", hr);
            return false;
        }
        cmd->Close();
        initialized = true;
        return true;
    }

    // Get-or-create the cached root signature + PSO for a kernel identity.
    CachedKernel* get_or_create_kernel(const uint8_t* dxil, size_t dxil_len,
                                       const uint8_t* rts0, size_t rts0_len,
                                       RxgdRecordResult* out) {
        uint64_t rts0_hash = fnv1a64(rts0, rts0_len);
        uint64_t key = fnv1a64(dxil, dxil_len) ^
                       ((rts0_hash << 1) | (rts0_hash >> 63));
        auto it = kernels.find(key);
        if (it != kernels.end()) return &it->second;

        CachedKernel k;
        // Sign an in-memory copy of the DXIL container so it can create a PSO on
        // a normal (non-Developer-Mode) device. The tracked artifact bytes the
        // bridge passed in are never modified.
        std::vector<uint8_t> signed_dxil(dxil, dxil + dxil_len);
        k.dxil_signed = sign_dxil_in_place(signed_dxil);
        HRESULT hr = device->CreateRootSignature(0, rts0, rts0_len, IID_PPV_ARGS(&k.root));
        if (FAILED(hr)) {
            set_detail(out, "CreateRootSignature(rurix rts0)", hr);
            return nullptr;
        }
        D3D12_COMPUTE_PIPELINE_STATE_DESC pd = {};
        pd.pRootSignature = k.root.Get();
        pd.CS = {signed_dxil.data(), signed_dxil.size()};
        hr = device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&k.pso));
        if (FAILED(hr)) {
            set_detail(out, "CreateComputePipelineState(rurix dxil)", hr);
            return nullptr;
        }
        auto res = kernels.emplace(key, std::move(k));
        return &res.first->second;
    }

    // Acquire the next allocator in the ring, blocking only if that specific
    // slot's previous submit has not completed yet (bounded recycle wait). This
    // is the production-path recycle: it never does a blanket fence wait.
    ID3D12CommandAllocator* acquire_allocator() {
        AllocatorSlot& slot = allocators[alloc_cursor];
        alloc_cursor = (alloc_cursor + 1) % allocators.size();
        if (slot.fence_value != 0 && fence->GetCompletedValue() < slot.fence_value) {
            fence->SetEventOnCompletion(slot.fence_value, fence_event);
            WaitForSingleObject(fence_event, INFINITE);
        }
        slot.alloc->Reset();
        return slot.alloc.Get();
    }

    // GRX-020: before reusing a wrapped descriptor range, wait for any
    // committed segment that overlaps [begin,end) whose submit has been signaled
    // but not yet completed, and prune segments that have completed. A segment
    // whose fence_value is beyond the last signaled value belongs to a record
    // that reserved descriptors but never submitted (a failed record); it is
    // safe to drop without waiting (never wait on an un-signaled fence — that
    // would deadlock).
    void wait_for_descriptor_range(UINT begin, UINT end) {
        const UINT64 signaled = next_fence_value;
        const UINT64 completed = fence ? fence->GetCompletedValue() : 0;
        UINT64 wait_value = 0;
        std::vector<DescriptorSegment> keep;
        keep.reserve(descriptor_ring.size());
        for (const DescriptorSegment& seg : descriptor_ring) {
            const bool overlaps = seg.begin < end && begin < seg.end;
            if (!overlaps) {
                // Keep only still-in-flight segments so the ring stays bounded.
                if (seg.fence_value > completed && seg.fence_value <= signaled) {
                    keep.push_back(seg);
                }
                continue;
            }
            // Overlapping range is being reused: wait for its submit if it is
            // signaled-but-incomplete; drop it either way.
            if (seg.fence_value <= signaled && seg.fence_value > completed) {
                wait_value = (std::max)(wait_value, seg.fence_value);
            }
        }
        if (wait_value != 0 && fence && fence_event) {
            if (fence->GetCompletedValue() < wait_value) {
                fence->SetEventOnCompletion(wait_value, fence_event);
                WaitForSingleObject(fence_event, INFINITE);
            }
            descriptor_ring_waits += 1;
        }
        descriptor_ring.swap(keep);
    }

    // Reserve `count` contiguous descriptors from the shader-visible heap,
    // returning the starting index. GRX-020: on wrap-around, wait (via
    // `wait_for_descriptor_range`) for any overlapping in-flight submit to
    // complete before handing the range back, then record the reservation
    // against the fence value the pending submit WILL signal
    // (`next_fence_value + 1`, since `next_fence_value` is only bumped at
    // Signal). All reserves within one record predict the same submit value,
    // and no single record reserves more than the heap capacity, so a record
    // never waits on its own unsubmitted fence.
    UINT reserve_descriptors(UINT count) {
        if (count > kDescriptorHeapCapacity) count = kDescriptorHeapCapacity;
        if (heap_cursor + count > kDescriptorHeapCapacity) {
            heap_cursor = 0;
            descriptor_ring_wraps += 1;
        }
        UINT base = heap_cursor;
        UINT end = base + count;
        wait_for_descriptor_range(base, end);
        heap_cursor = end;
        descriptor_ring.push_back(DescriptorSegment{base, end, next_fence_value + 1});
        return base;
    }

    D3D12_CPU_DESCRIPTOR_HANDLE cpu_handle(UINT index) const {
        D3D12_CPU_DESCRIPTOR_HANDLE h = heap->GetCPUDescriptorHandleForHeapStart();
        h.ptr += (SIZE_T)index * descriptor_increment;
        return h;
    }
    D3D12_GPU_DESCRIPTOR_HANDLE gpu_handle(UINT index) const {
        D3D12_GPU_DESCRIPTOR_HANDLE h = heap->GetGPUDescriptorHandleForHeapStart();
        h.ptr += (UINT64)index * descriptor_increment;
        return h;
    }

    void note(uint32_t pass_id, bool recorded) {
        PassCounters& c = counters[pass_id];
        if (recorded) c.recorded += 1; else c.fallback += 1;
        // Periodic engagement flush so a force-quit (session never closed) still
        // leaves a recent on-disk count. Best-effort; no-op without the env var.
        if (++note_count % kEngagementFlushInterval == 0) {
            flush_engagement();
        }
    }

    // Serialize the per-pass recorded/fallback counters to the engagement JSON
    // file (RXGD_ENGAGEMENT_OUTPUT). Best-effort and a no-op when the env var is
    // unset. Format: {"<pass_id>":{"recorded":n,"fallback":m}, ...}.
    void flush_engagement() const {
        const std::string& path = engagement_output_path();
        if (path.empty()) return;
        std::string json = "{";
        bool first = true;
        for (const auto& kv : counters) {
            if (!first) json += ",";
            first = false;
            char entry[128];
            std::snprintf(entry, sizeof(entry),
                          "\"%u\":{\"recorded\":%llu,\"fallback\":%llu}",
                          kv.first, (unsigned long long)kv.second.recorded,
                          (unsigned long long)kv.second.fallback);
            json += entry;
        }
        json += "}\n";
        atomic_write_engagement(path, json);
    }

    // Print one machine-readable summary line per pass touched this session, and
    // write the final engagement file. Called at session close.
    void print_summary() {
        for (const auto& kv : counters) {
            std::printf("RXGD_SUMMARY pass=%u recorded=%llu fallback=%llu\n",
                        kv.first, (unsigned long long)kv.second.recorded,
                        (unsigned long long)kv.second.fallback);
        }
        // GRX-020 descriptor-ring telemetry (heap-segment reclaim health): how
        // often the shader-visible descriptor cursor wrapped and how often a
        // wrap had to wait for an in-flight submit to finish before reusing a
        // range. Pure telemetry — the ring is a hardening, not a pass.
        std::printf("RXGD_DESCRIPTOR_RING wraps=%llu waits=%llu capacity=%u\n",
                    (unsigned long long)descriptor_ring_wraps,
                    (unsigned long long)descriptor_ring_waits,
                    kDescriptorHeapCapacity);
        std::fflush(stdout);
        flush_engagement();
    }

    // Core multi-level record. `kernels`/`resources`/`levels` are borrowed. On
    // success returns 0 and fills `out`; on any D3D12 failure returns a negative
    // status with `out->error_detail` set. `readback` selects test-only readback
    // + fence wait + RXGD_BRIDGE_REC marker (true) vs production no-wait (false).
    int record_levels(uint32_t pass_id,
                      const RxgdShimKernel* kern, uint32_t kernel_count,
                      const RxgdShimResource* resources, uint32_t resource_count,
                      const RxgdShimLevel* levels, uint32_t level_count,
                      bool readback, RxgdRecordResult* out) {
        if (level_count == 0) {
            set_detail_msg(out, "record_levels called with zero levels");
            return -30;
        }
        if (!ensure_initialized(out)) return -31;

        // Resolve + cache every kernel referenced by the levels up front.
        std::vector<CachedKernel*> resolved(kernel_count, nullptr);
        bool all_signed = true;
        for (uint32_t i = 0; i < kernel_count; ++i) {
            resolved[i] = get_or_create_kernel(kern[i].dxil, kern[i].dxil_len,
                                               kern[i].rts0, kern[i].rts0_len, out);
            if (!resolved[i]) return -32;
            all_signed = all_signed && resolved[i]->dxil_signed;
        }

        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) {
            set_detail(out, "command list Reset", hr);
            return -33;
        }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);

        // Per-resource logical D3D12 state, tracked so inter-level role changes
        // (a level's UAV dst read as the next level's SRV src) get a correct
        // transition barrier. Seed each resource with the state implied by its
        // FIRST use across levels (caller contract: provide SRV sources in
        // NON_PIXEL_SHADER_RESOURCE and UAV dests in UNORDERED_ACCESS).
        std::vector<D3D12_RESOURCE_STATES> state(resource_count,
                                                 D3D12_RESOURCE_STATE_COMMON);
        std::vector<bool> seeded(resource_count, false);
        auto seed = [&](uint32_t idx, D3D12_RESOURCE_STATES s) {
            if (idx < resource_count && !seeded[idx]) {
                state[idx] = s;
                seeded[idx] = true;
            }
        };
        for (uint32_t li = 0; li < level_count; ++li) {
            const RxgdShimLevel& lv = levels[li];
            const RxgdShimKernel& kb = kern[lv.kernel_index];
            seed(lv.srv_index, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
            if (kb.binding_count == 3)
                seed(lv.prev_index, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
            seed(lv.uav_index, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        }

        auto transition = [&](uint32_t idx, D3D12_RESOURCE_STATES after) {
            if (idx >= resource_count) return;
            if (state[idx] == after) return;
            D3D12_RESOURCE_BARRIER b = {};
            b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
            b.Transition.pResource =
                reinterpret_cast<ID3D12Resource*>(resources[idx].resource);
            b.Transition.StateBefore = state[idx];
            b.Transition.StateAfter = after;
            b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
            cmd->ResourceBarrier(1, &b);
            state[idx] = after;
        };

        uint32_t last_uav_index = levels[level_count - 1].uav_index;
        UINT last_dst_w = std::max<UINT>(levels[level_count - 1].dst_width, 1u);
        UINT last_dst_h = std::max<UINT>(levels[level_count - 1].dst_height, 1u);
        UINT last_gx = 0, last_gy = 0, last_gz = 0;

        for (uint32_t li = 0; li < level_count; ++li) {
            const RxgdShimLevel& lv = levels[li];
            const RxgdShimKernel& kb = kern[lv.kernel_index];
            CachedKernel* k = resolved[lv.kernel_index];

            ID3D12Resource* src =
                reinterpret_cast<ID3D12Resource*>(resources[lv.srv_index].resource);
            ID3D12Resource* dst =
                reinterpret_cast<ID3D12Resource*>(resources[lv.uav_index].resource);
            ID3D12Resource* prev =
                (kb.binding_count == 3)
                    ? reinterpret_cast<ID3D12Resource*>(resources[lv.prev_index].resource)
                    : nullptr;

            // Fail-closed BEFORE issuing any view: if a bound resource's typeless
            // family is not one this shim maps to a typed member, creating the
            // view would be an invalid D3D12 call that removes the device. Refuse
            // and let the bridge fall back to the native Godot path instead.
            if (format_is_typeless(typed_view_format(src->GetDesc().Format)) ||
                format_is_typeless(typed_view_format(dst->GetDesc().Format)) ||
                (prev && format_is_typeless(typed_view_format(prev->GetDesc().Format)))) {
                set_detail_msg(out, "bound resource carries an unmapped typeless format");
                return -40;
            }

            // Ensure inputs/outputs are in the right state for this level.
            transition(lv.srv_index, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
            if (prev)
                transition(lv.prev_index, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
            transition(lv.uav_index, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);

            // Reserve + write the descriptor table for this level.
            UINT base = reserve_descriptors(kb.binding_count);
            UINT slot = base;
            D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
            srv.Format = typed_view_format(src->GetDesc().Format);
            srv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
            srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
            srv.Texture2D.MipLevels = 1;
            device->CreateShaderResourceView(src, &srv, cpu_handle(slot++));
            if (prev) {
                D3D12_SHADER_RESOURCE_VIEW_DESC psrv = {};
                psrv.Format = typed_view_format(prev->GetDesc().Format);
                psrv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
                psrv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
                psrv.Texture2D.MipLevels = 1;
                device->CreateShaderResourceView(prev, &psrv, cpu_handle(slot++));
            }
            D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
            uav.Format = typed_view_format(dst->GetDesc().Format);
            uav.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
            device->CreateUnorderedAccessView(dst, nullptr, &uav, cpu_handle(slot++));

            cmd->SetComputeRootSignature(k->root.Get());
            cmd->SetPipelineState(k->pso.Get());
            uint32_t rc[7];
            std::memcpy(rc, lv.push_constants, 28);
            cmd->SetComputeRoot32BitConstants(0, 7, rc, 0);
            cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
            const UINT gx = std::max<UINT>(lv.dispatch_x, 1u);
            const UINT gy = std::max<UINT>(lv.dispatch_y, 1u);
            const UINT gz = std::max<UINT>(lv.dispatch_z, 1u);
            cmd->Dispatch(gx, gy, gz);
            last_gx = gx; last_gy = gy; last_gz = gz;

            // Inter-level ordering: a UAV barrier on this level's dst so a later
            // level that reads it (as SRV, after a transition) observes the
            // completed writes.
            if (li + 1 < level_count) {
                D3D12_RESOURCE_BARRIER ub = {};
                ub.Type = D3D12_RESOURCE_BARRIER_TYPE_UAV;
                ub.UAV.pResource = dst;
                cmd->ResourceBarrier(1, &ub);
            }
        }

        // Optional test-only readback of the final level's dst UAV.
        ComPtr<ID3D12Resource> readback_buf;
        D3D12_PLACED_SUBRESOURCE_FOOTPRINT dfp = {};
        UINT64 dtotal = 0;
        DXGI_FORMAT rb_format = DXGI_FORMAT_UNKNOWN;
        if (readback) {
            ID3D12Resource* dst = reinterpret_cast<ID3D12Resource*>(
                resources[last_uav_index].resource);
            D3D12_RESOURCE_DESC dst_desc = dst->GetDesc();
            // Clean single-mip 2D footprint desc (Godot resources can carry an
            // Alignment/Flags combination that GetCopyableFootprints rejects,
            // returning UINT64_MAX). Only typed format + extent matter here.
            D3D12_RESOURCE_DESC footprint_desc = {};
            footprint_desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
            footprint_desc.Alignment = 0;
            footprint_desc.Width = dst_desc.Width;
            footprint_desc.Height = dst_desc.Height;
            footprint_desc.DepthOrArraySize = 1;
            footprint_desc.MipLevels = 1;
            footprint_desc.Format = typed_view_format(dst_desc.Format);
            rb_format = footprint_desc.Format;
            footprint_desc.SampleDesc.Count = 1;
            footprint_desc.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
            footprint_desc.Flags = D3D12_RESOURCE_FLAG_NONE;
            UINT drows = 0;
            UINT64 drow_size = 0;
            device->GetCopyableFootprints(&footprint_desc, 0, 1, 0, &dfp, &drows,
                                          &drow_size, &dtotal);
            if (dtotal == 0 || dtotal == UINT64_MAX) {
                set_detail_msg(out, "GetCopyableFootprints returned an invalid total size");
                return -34;
            }
            auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);
            auto rb_desc = buffer_desc(dtotal);
            hr = device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE,
                                                 &rb_desc, D3D12_RESOURCE_STATE_COPY_DEST,
                                                 nullptr, IID_PPV_ARGS(&readback_buf));
            if (FAILED(hr)) {
                set_detail(out, "CreateCommittedResource(readback)", hr);
                return -34;
            }
            transition(last_uav_index, D3D12_RESOURCE_STATE_COPY_SOURCE);
            D3D12_TEXTURE_COPY_LOCATION cdst = {};
            cdst.pResource = readback_buf.Get();
            cdst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
            cdst.PlacedFootprint = dfp;
            D3D12_TEXTURE_COPY_LOCATION csrc = {};
            csrc.pResource = dst;
            csrc.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
            csrc.SubresourceIndex = 0;
            cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
            // Restore the final dst to UNORDERED_ACCESS so the caller's resource
            // state is unchanged (matches the historical single-level contract).
            transition(last_uav_index, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        }

        hr = cmd->Close();
        if (FAILED(hr)) {
            set_detail(out, "Close command list", hr);
            return -35;
        }
        ID3D12CommandList* lists[] = {cmd.Get()};
        queue->ExecuteCommandLists(1, lists);
        UINT64 submit_value = ++next_fence_value;
        if (FAILED(queue->Signal(fence.Get(), submit_value))) {
            set_detail_msg(out, "Signal fence");
            return -36;
        }
        // Record the fence value against the allocator slot we used, so the ring
        // recycle can reuse it once complete.
        allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value =
            submit_value;

        out->fence_completed_value = submit_value;
        out->dispatch_x = last_gx;
        out->dispatch_y = last_gy;
        out->dispatch_z = last_gz;
        out->dst_width = last_dst_w;
        out->dst_height = last_dst_h;
        out->dxil_signed = all_signed ? 1 : 0;

        if (!readback) {
            // Production path: DO NOT block. The dispatch is submitted; the fence
            // value is tracked for allocator recycling. No per-frame marker.
            out->readback_checksum = 0;
            out->dst_first_value = 0.0f;
            std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
            return 0;
        }

        // Test-only readback path: wait for completion, checksum, marker.
        if (fence->GetCompletedValue() < submit_value) {
            if (FAILED(fence->SetEventOnCompletion(submit_value, fence_event))) {
                set_detail_msg(out, "SetEventOnCompletion");
                return -37;
            }
            WaitForSingleObject(fence_event, INFINITE);
        }
        out->fence_completed_value = fence->GetCompletedValue();
        if (out->fence_completed_value < submit_value) {
            set_detail_msg(out, "fence did not reach completion");
            return -38;
        }

        uint8_t* mapped = nullptr;
        D3D12_RANGE range = {0, (SIZE_T)dtotal};
        if (FAILED(readback_buf->Map(0, &range, reinterpret_cast<void**>(&mapped)))) {
            set_detail_msg(out, "Map readback");
            return -39;
        }
        // Checksum the copied dst rows. The per-pixel stride MUST follow the real
        // format's bytes-per-pixel: the old shim assumed a fixed 4-byte stride,
        // which reads past the actual per-row footprint (and past the mapped
        // buffer's last row) for narrow formats such as the SSAO R8G8 (2 bytes)
        // AO buffers, an out-of-bounds read that access-violates. `bpp==0` means
        // a format this shim does not size, so fall back to a defensive 1-byte
        // walk. Every read is additionally clamped to the mapped buffer end.
        UINT bpp = dxgi_format_bytes(rb_format);
        if (bpp == 0) bpp = 1;
        const UINT row_pitch = dfp.Footprint.RowPitch;
        const UINT cols_in_pitch = row_pitch / bpp;  // pixels that actually fit
        const UINT rows = std::min<UINT>(last_dst_h, dfp.Footprint.Height);
        const UINT cols = std::min<UINT>(last_dst_w, cols_in_pitch);
        const uint8_t* const map_end = mapped + (SIZE_T)dtotal;
        const UINT sample_bytes = std::min<UINT>(bpp, 4u);
        uint32_t checksum = 2166136261u;  // FNV-1a over the dst rows
        float first = 0.0f;
        bool got_first = false;
        for (UINT y = 0; y < rows; ++y) {
            const uint8_t* rowp = mapped + dfp.Offset + (SIZE_T)y * row_pitch;
            for (UINT x = 0; x < cols; ++x) {
                const uint8_t* px = rowp + (SIZE_T)x * bpp;
                if (px + sample_bytes > map_end) {
                    break;  // never read past the mapped readback buffer
                }
                if (!got_first) {
                    std::memcpy(&first, px, sample_bytes);
                    got_first = true;
                }
                for (UINT b = 0; b < sample_bytes; ++b) {
                    checksum ^= px[b];
                    checksum *= 16777619u;
                }
            }
        }
        readback_buf->Unmap(0, nullptr);

        out->readback_checksum = checksum;
        out->dst_first_value = first;
        std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
        std::printf("RXGD_BRIDGE_REC: dispatch=%u,%u,%u fence=%llu dst=%ux%u dst_first=%g "
                    "checksum=0x%08x dxil_signed=%s\n",
                    last_gx, last_gy, last_gz,
                    (unsigned long long)out->fence_completed_value, last_dst_w, last_dst_h,
                    first, checksum, out->dxil_signed ? "yes" : "no");
        std::fflush(stdout);
        return 0;
    }

    // ── GRX-012: 5-SRV (t0..t4) + 1-UAV (u0) single-dispatch record ──────────
    // The taa_resolve kernel binds five Texture2D SRVs and one RWTexture2D UAV,
    // which the srv/prev/uav RxgdShimLevel record path cannot express. This
    // dedicated method records one dispatch; `readback` selects the test/
    // recording instrumented mode (fence wait + checksum + RXGD_BRIDGE_REC
    // marker) vs the production mode (submit only, no readback / marker),
    // reusing the session kernel cache / allocator ring / descriptor heap /
    // fence. The luminance / tonemap / ssao_blur record paths are untouched.
    int record_taa(uint32_t pass_id,
                   const uint8_t* dxil, size_t dxil_len,
                   const uint8_t* rts0, size_t rts0_len,
                   ID3D12Resource* const srvs[5], ID3D12Resource* uav,
                   const uint8_t* push_constants,
                   uint32_t width, uint32_t height,
                   bool readback, RxgdRecordResult* out) {
        (void)pass_id;
        if (!ensure_initialized(out)) return -31;
        CachedKernel* k = get_or_create_kernel(dxil, dxil_len, rts0, rts0_len, out);
        if (!k) return -32;

        // Fail-closed typeless guard on all six bound resources: creating a view
        // with an unmapped typeless format is an invalid D3D12 call that removes
        // the device; refuse and let the bridge fall back.
        ID3D12Resource* all[6] = {srvs[0], srvs[1], srvs[2], srvs[3], srvs[4], uav};
        for (int i = 0; i < 6; ++i) {
            if (!all[i]) { set_detail_msg(out, "null taa resource handle"); return -41; }
            const DXGI_FORMAT res_fmt = all[i]->GetDesc().Format;
            if (format_is_typeless(typed_view_format(res_fmt))) {
                std::snprintf(out->error_detail, sizeof(out->error_detail),
                              "bound resource carries an unmapped typeless format "
                              "(slot=%d resource_format=%d typed_view=%d)",
                              i, (int)res_fmt, (int)typed_view_format(res_fmt));
                return -40;
            }
        }

        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) { set_detail(out, "command list Reset", hr); return -33; }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);

        // Descriptor table: SRV range (t0..t4) then UAV range (u0), contiguous.
        UINT base = reserve_descriptors(6);
        UINT slot = base;
        for (int i = 0; i < 5; ++i) {
            D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
            srv.Format = typed_view_format(srvs[i]->GetDesc().Format);
            srv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
            srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
            srv.Texture2D.MipLevels = 1;
            device->CreateShaderResourceView(srvs[i], &srv, cpu_handle(slot++));
        }
        D3D12_UNORDERED_ACCESS_VIEW_DESC uavd = {};
        uavd.Format = typed_view_format(uav->GetDesc().Format);
        uavd.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
        device->CreateUnorderedAccessView(uav, nullptr, &uavd, cpu_handle(slot++));

        cmd->SetComputeRootSignature(k->root.Get());
        cmd->SetPipelineState(k->pso.Get());
        uint32_t rc[7];
        std::memcpy(rc, push_constants, 28);
        cmd->SetComputeRoot32BitConstants(0, 7, rc, 0);
        cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
        const UINT gx = std::max<UINT>((width + 7) / 8, 1u);
        const UINT gy = std::max<UINT>((height + 7) / 8, 1u);
        cmd->Dispatch(gx, gy, 1);

        if (!readback) {
            // Production path: submit the dispatch and track the fence value for
            // allocator recycling; NO copy / fence-wait / map / checksum / marker.
            hr = cmd->Close();
            if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
            ID3D12CommandList* plists[] = {cmd.Get()};
            queue->ExecuteCommandLists(1, plists);
            UINT64 pval = ++next_fence_value;
            if (FAILED(queue->Signal(fence.Get(), pval))) {
                set_detail_msg(out, "Signal fence");
                return -36;
            }
            allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()]
                .fence_value = pval;
            out->fence_completed_value = pval;
            out->dispatch_x = gx;
            out->dispatch_y = gy;
            out->dispatch_z = 1;
            out->dst_width = width;
            out->dst_height = height;
            out->readback_checksum = 0;
            out->dst_first_value = 0.0f;
            out->dxil_signed = k->dxil_signed ? 1 : 0;
            std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
            return 0;
        }

        // Test-only readback of the output UAV.
        D3D12_RESOURCE_DESC uav_desc = uav->GetDesc();
        D3D12_RESOURCE_DESC footprint_desc = {};
        footprint_desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
        footprint_desc.Width = uav_desc.Width;
        footprint_desc.Height = uav_desc.Height;
        footprint_desc.DepthOrArraySize = 1;
        footprint_desc.MipLevels = 1;
        footprint_desc.Format = typed_view_format(uav_desc.Format);
        footprint_desc.SampleDesc.Count = 1;
        footprint_desc.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
        DXGI_FORMAT rb_format = footprint_desc.Format;
        D3D12_PLACED_SUBRESOURCE_FOOTPRINT dfp = {};
        UINT drows = 0;
        UINT64 drow_size = 0, dtotal = 0;
        device->GetCopyableFootprints(&footprint_desc, 0, 1, 0, &dfp, &drows, &drow_size, &dtotal);
        if (dtotal == 0 || dtotal == UINT64_MAX) {
            set_detail_msg(out, "GetCopyableFootprints returned an invalid total size");
            return -34;
        }
        auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
        auto rb_desc = buffer_desc(dtotal);
        ComPtr<ID3D12Resource> readback_buf;
        hr = device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                             D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                             IID_PPV_ARGS(&readback_buf));
        if (FAILED(hr)) { set_detail(out, "CreateCommittedResource(readback)", hr); return -34; }

        D3D12_RESOURCE_BARRIER tb = {};
        tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        tb.Transition.pResource = uav;
        tb.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        tb.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &tb);
        D3D12_TEXTURE_COPY_LOCATION cdst = {};
        cdst.pResource = readback_buf.Get();
        cdst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
        cdst.PlacedFootprint = dfp;
        D3D12_TEXTURE_COPY_LOCATION csrc = {};
        csrc.pResource = uav;
        csrc.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        csrc.SubresourceIndex = 0;
        cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
        D3D12_RESOURCE_BARRIER tb2 = tb;
        tb2.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb2.Transition.StateAfter = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        cmd->ResourceBarrier(1, &tb2);

        hr = cmd->Close();
        if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
        ID3D12CommandList* lists[] = {cmd.Get()};
        queue->ExecuteCommandLists(1, lists);
        UINT64 submit_value = ++next_fence_value;
        if (FAILED(queue->Signal(fence.Get(), submit_value))) {
            set_detail_msg(out, "Signal fence");
            return -36;
        }
        allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value =
            submit_value;

        if (fence->GetCompletedValue() < submit_value) {
            if (FAILED(fence->SetEventOnCompletion(submit_value, fence_event))) {
                set_detail_msg(out, "SetEventOnCompletion");
                return -37;
            }
            WaitForSingleObject(fence_event, INFINITE);
        }
        out->fence_completed_value = fence->GetCompletedValue();
        if (out->fence_completed_value < submit_value) {
            set_detail_msg(out, "fence did not reach completion");
            return -38;
        }

        uint8_t* mapped = nullptr;
        D3D12_RANGE rng = {0, (SIZE_T)dtotal};
        if (FAILED(readback_buf->Map(0, &rng, reinterpret_cast<void**>(&mapped)))) {
            set_detail_msg(out, "Map readback");
            return -39;
        }
        UINT bpp = dxgi_format_bytes(rb_format);
        if (bpp == 0) bpp = 1;
        const UINT row_pitch = dfp.Footprint.RowPitch;
        const UINT cols_in_pitch = row_pitch / bpp;
        const UINT rows = std::min<UINT>(height, dfp.Footprint.Height);
        const UINT cols = std::min<UINT>(width, cols_in_pitch);
        const uint8_t* const map_end = mapped + (SIZE_T)dtotal;
        const UINT sample_bytes = std::min<UINT>(bpp, 4u);
        uint32_t checksum = 2166136261u;
        float first = 0.0f;
        bool got_first = false;
        for (UINT y = 0; y < rows; ++y) {
            const uint8_t* rowp = mapped + dfp.Offset + (SIZE_T)y * row_pitch;
            for (UINT x = 0; x < cols; ++x) {
                const uint8_t* px = rowp + (SIZE_T)x * bpp;
                if (px + sample_bytes > map_end) break;
                if (!got_first) {
                    std::memcpy(&first, px, sample_bytes);
                    got_first = true;
                }
                for (UINT b = 0; b < sample_bytes; ++b) {
                    checksum ^= px[b];
                    checksum *= 16777619u;
                }
            }
        }
        readback_buf->Unmap(0, nullptr);

        out->dispatch_x = gx;
        out->dispatch_y = gy;
        out->dispatch_z = 1;
        out->dst_width = width;
        out->dst_height = height;
        out->readback_checksum = checksum;
        out->dst_first_value = first;
        out->dxil_signed = k->dxil_signed ? 1 : 0;
        std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
        std::printf("RXGD_BRIDGE_REC: dispatch=%u,%u,%u fence=%llu dst=%ux%u dst_first=%g "
                    "checksum=0x%08x dxil_signed=%s\n",
                    gx, gy, 1u, (unsigned long long)out->fence_completed_value, width, height,
                    first, checksum, out->dxil_signed ? "yes" : "no");
        std::fflush(stdout);
        return 0;
    }

    // GRX-013 particles_copy: StructuredBuffer SRV (t0) + RWStructuredBuffer UAV
    // (u0) + 128-byte (32-dword) CopyPushConstant b0 root constants, dispatch
    // ceil(total_particles / 64). Test-only readback of the destination
    // structured buffer via CopyBufferRegion. `src_bytes`/`dst_bytes` are the
    // buffer byte sizes; the ParticleData stride is 112 (source) and the
    // instance stride is 16 (float4, destination).
    int record_particles_copy(uint32_t pass_id,
                              const uint8_t* dxil, size_t dxil_len,
                              const uint8_t* rts0, size_t rts0_len,
                              ID3D12Resource* src_particles, ID3D12Resource* dst_instances,
                              const uint8_t* push_constants,
                              uint32_t src_bytes, uint32_t dst_bytes,
                              bool readback, RxgdRecordResult* out) {
        (void)pass_id;
        if (!ensure_initialized(out)) return -31;
        CachedKernel* k = get_or_create_kernel(dxil, dxil_len, rts0, rts0_len, out);
        if (!k) return -32;
        if (!src_particles || !dst_instances) {
            set_detail_msg(out, "null particles_copy buffer handle");
            return -41;
        }

        // ParticleData stride = 112 bytes (source); float4 instance stride = 16
        // bytes (destination). NumElements is clamped to at least 1.
        const UINT src_stride = 112u;
        const UINT dst_stride = 16u;
        const UINT src_elements = std::max<UINT>(src_bytes / src_stride, 1u);
        const UINT dst_elements = std::max<UINT>(dst_bytes / dst_stride, 1u);

        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) { set_detail(out, "command list Reset", hr); return -33; }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);

        // Descriptor table: SRV range (t0) then UAV range (u0), contiguous.
        UINT base = reserve_descriptors(2);
        UINT slot = base;
        {
            D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
            srv.Format = DXGI_FORMAT_UNKNOWN; // structured buffer
            srv.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
            srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
            srv.Buffer.FirstElement = 0;
            srv.Buffer.NumElements = src_elements;
            srv.Buffer.StructureByteStride = src_stride;
            srv.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
            device->CreateShaderResourceView(src_particles, &srv, cpu_handle(slot++));
        }
        {
            D3D12_UNORDERED_ACCESS_VIEW_DESC uavd = {};
            uavd.Format = DXGI_FORMAT_UNKNOWN; // rwstructured buffer
            uavd.ViewDimension = D3D12_UAV_DIMENSION_BUFFER;
            uavd.Buffer.FirstElement = 0;
            uavd.Buffer.NumElements = dst_elements;
            uavd.Buffer.StructureByteStride = dst_stride;
            uavd.Buffer.CounterOffsetInBytes = 0;
            uavd.Buffer.Flags = D3D12_BUFFER_UAV_FLAG_NONE;
            device->CreateUnorderedAccessView(dst_instances, nullptr, &uavd, cpu_handle(slot++));
        }

        cmd->SetComputeRootSignature(k->root.Get());
        cmd->SetPipelineState(k->pso.Get());
        uint32_t rc[32];
        std::memcpy(rc, push_constants, 128);
        cmd->SetComputeRoot32BitConstants(0, 32, rc, 0);
        cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
        const uint32_t total_particles = rc[3]; // CopyPushConstant dword 3
        const UINT gx = std::max<UINT>((total_particles + 63u) / 64u, 1u);
        cmd->Dispatch(gx, 1, 1);

        if (!readback) {
            // Production path: submit + track the fence value; NO copy /
            // fence-wait / map / checksum / marker.
            hr = cmd->Close();
            if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
            ID3D12CommandList* plists[] = {cmd.Get()};
            queue->ExecuteCommandLists(1, plists);
            UINT64 pval = ++next_fence_value;
            if (FAILED(queue->Signal(fence.Get(), pval))) {
                set_detail_msg(out, "Signal fence");
                return -36;
            }
            allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()]
                .fence_value = pval;
            out->fence_completed_value = pval;
            out->dispatch_x = gx;
            out->dispatch_y = 1;
            out->dispatch_z = 1;
            out->dst_width = dst_bytes;
            out->dst_height = 1;
            out->readback_checksum = 0;
            out->dst_first_value = 0.0f;
            out->dxil_signed = k->dxil_signed ? 1 : 0;
            std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
            return 0;
        }

        // Test-only readback of the destination structured buffer.
        const UINT64 rb_bytes = (UINT64)dst_elements * dst_stride;
        auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
        auto rb_desc = buffer_desc(rb_bytes);
        ComPtr<ID3D12Resource> readback_buf;
        hr = device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                             D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                             IID_PPV_ARGS(&readback_buf));
        if (FAILED(hr)) { set_detail(out, "CreateCommittedResource(readback)", hr); return -34; }

        D3D12_RESOURCE_BARRIER tb = {};
        tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        tb.Transition.pResource = dst_instances;
        tb.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        tb.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &tb);
        cmd->CopyBufferRegion(readback_buf.Get(), 0, dst_instances, 0, rb_bytes);
        D3D12_RESOURCE_BARRIER tb2 = tb;
        tb2.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb2.Transition.StateAfter = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        cmd->ResourceBarrier(1, &tb2);

        hr = cmd->Close();
        if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
        ID3D12CommandList* lists[] = {cmd.Get()};
        queue->ExecuteCommandLists(1, lists);
        UINT64 submit_value = ++next_fence_value;
        if (FAILED(queue->Signal(fence.Get(), submit_value))) {
            set_detail_msg(out, "Signal fence");
            return -36;
        }
        allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value =
            submit_value;

        if (fence->GetCompletedValue() < submit_value) {
            if (FAILED(fence->SetEventOnCompletion(submit_value, fence_event))) {
                set_detail_msg(out, "SetEventOnCompletion");
                return -37;
            }
            WaitForSingleObject(fence_event, INFINITE);
        }
        out->fence_completed_value = fence->GetCompletedValue();
        if (out->fence_completed_value < submit_value) {
            set_detail_msg(out, "fence did not reach completion");
            return -38;
        }

        uint8_t* mapped = nullptr;
        D3D12_RANGE rng = {0, (SIZE_T)rb_bytes};
        if (FAILED(readback_buf->Map(0, &rng, reinterpret_cast<void**>(&mapped)))) {
            set_detail_msg(out, "Map readback");
            return -39;
        }
        uint32_t checksum = 2166136261u;
        float first = 0.0f;
        if (rb_bytes >= sizeof(float)) {
            std::memcpy(&first, mapped, sizeof(float));
        }
        for (UINT64 i = 0; i < rb_bytes; ++i) {
            checksum ^= mapped[i];
            checksum *= 16777619u;
        }
        readback_buf->Unmap(0, nullptr);

        out->dispatch_x = gx;
        out->dispatch_y = 1;
        out->dispatch_z = 1;
        out->dst_width = dst_bytes;
        out->dst_height = 1;
        out->readback_checksum = checksum;
        out->dst_first_value = first;
        out->dxil_signed = k->dxil_signed ? 1 : 0;
        std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
        std::printf("RXGD_BRIDGE_REC: dispatch=%u,1,1 fence=%llu dst_bytes=%u dst_first=%g "
                    "checksum=0x%08x dxil_signed=%s\n",
                    gx, (unsigned long long)out->fence_completed_value, dst_bytes,
                    first, checksum, out->dxil_signed ? "yes" : "no");
        std::fflush(stdout);
        return 0;
    }

    // GRX-014 cluster_store: StructuredBuffer SRV t0 (cluster_render, uint
    // words) + StructuredBuffer SRV t1 (render_elements, 80-byte
    // RenderElementData) + RWStructuredBuffer UAV u0 (cluster_store, uint
    // words) + 32-byte (8-dword) ClusterStore::PushConstant b0 root constants,
    // dispatch ceil(cluster_screen_size.x / 8) x ceil(cluster_screen_size.y
    // / 8) where cluster_screen_size is b0 dwords 2-3. Test-only readback of
    // the destination structured buffer via CopyBufferRegion; the production
    // path (readback=false) submits without any copy / fence-wait / map /
    // checksum / marker. `*_bytes` are the buffer byte sizes.
    int record_cluster_store(uint32_t pass_id,
                             const uint8_t* dxil, size_t dxil_len,
                             const uint8_t* rts0, size_t rts0_len,
                             ID3D12Resource* cluster_render_buf,
                             ID3D12Resource* render_elements_buf,
                             ID3D12Resource* cluster_store_buf,
                             const uint8_t* push_constants,
                             uint32_t cluster_render_bytes,
                             uint32_t render_elements_bytes,
                             uint32_t cluster_store_bytes,
                             bool readback, RxgdRecordResult* out) {
        (void)pass_id;
        if (!ensure_initialized(out)) return -31;
        CachedKernel* k = get_or_create_kernel(dxil, dxil_len, rts0, rts0_len, out);
        if (!k) return -32;
        if (!cluster_render_buf || !render_elements_buf || !cluster_store_buf) {
            set_detail_msg(out, "null cluster_store buffer handle");
            return -41;
        }

        // uint word stride = 4 bytes (cluster_render / cluster_store);
        // RenderElementData stride = 80 bytes (render_elements). NumElements is
        // clamped to at least 1.
        const UINT word_stride = 4u;
        const UINT element_stride = 80u;
        const UINT render_words = std::max<UINT>(cluster_render_bytes / word_stride, 1u);
        const UINT element_count = std::max<UINT>(render_elements_bytes / element_stride, 1u);
        const UINT store_words = std::max<UINT>(cluster_store_bytes / word_stride, 1u);

        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) { set_detail(out, "command list Reset", hr); return -33; }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);

        // Descriptor table: SRV range (t0, t1) then UAV range (u0), contiguous.
        UINT base = reserve_descriptors(3);
        UINT slot = base;
        {
            D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
            srv.Format = DXGI_FORMAT_UNKNOWN; // structured buffer
            srv.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
            srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
            srv.Buffer.FirstElement = 0;
            srv.Buffer.NumElements = render_words;
            srv.Buffer.StructureByteStride = word_stride;
            srv.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
            device->CreateShaderResourceView(cluster_render_buf, &srv, cpu_handle(slot++));
        }
        {
            D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
            srv.Format = DXGI_FORMAT_UNKNOWN; // structured buffer
            srv.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
            srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
            srv.Buffer.FirstElement = 0;
            srv.Buffer.NumElements = element_count;
            srv.Buffer.StructureByteStride = element_stride;
            srv.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
            device->CreateShaderResourceView(render_elements_buf, &srv, cpu_handle(slot++));
        }
        {
            D3D12_UNORDERED_ACCESS_VIEW_DESC uavd = {};
            uavd.Format = DXGI_FORMAT_UNKNOWN; // rwstructured buffer
            uavd.ViewDimension = D3D12_UAV_DIMENSION_BUFFER;
            uavd.Buffer.FirstElement = 0;
            uavd.Buffer.NumElements = store_words;
            uavd.Buffer.StructureByteStride = word_stride;
            uavd.Buffer.CounterOffsetInBytes = 0;
            uavd.Buffer.Flags = D3D12_BUFFER_UAV_FLAG_NONE;
            device->CreateUnorderedAccessView(cluster_store_buf, nullptr, &uavd,
                                              cpu_handle(slot++));
        }

        cmd->SetComputeRootSignature(k->root.Get());
        cmd->SetPipelineState(k->pso.Get());
        uint32_t rc[8];
        std::memcpy(rc, push_constants, 32);
        cmd->SetComputeRoot32BitConstants(0, 8, rc, 0);
        cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
        const uint32_t screen_x = rc[2]; // ClusterStore::PushConstant dword 2
        const uint32_t screen_y = rc[3]; // ClusterStore::PushConstant dword 3
        const UINT gx = std::max<UINT>((screen_x + 7u) / 8u, 1u);
        const UINT gy = std::max<UINT>((screen_y + 7u) / 8u, 1u);
        cmd->Dispatch(gx, gy, 1);

        if (!readback) {
            // Production path: submit + track the fence value; NO copy /
            // fence-wait / map / checksum / marker.
            hr = cmd->Close();
            if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
            ID3D12CommandList* plists[] = {cmd.Get()};
            queue->ExecuteCommandLists(1, plists);
            UINT64 pval = ++next_fence_value;
            if (FAILED(queue->Signal(fence.Get(), pval))) {
                set_detail_msg(out, "Signal fence");
                return -36;
            }
            allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()]
                .fence_value = pval;
            out->fence_completed_value = pval;
            out->dispatch_x = gx;
            out->dispatch_y = gy;
            out->dispatch_z = 1;
            out->dst_width = cluster_store_bytes;
            out->dst_height = 1;
            out->readback_checksum = 0;
            out->dst_first_value = 0.0f;
            out->dxil_signed = k->dxil_signed ? 1 : 0;
            std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
            return 0;
        }

        // Test-only readback of the destination structured buffer.
        const UINT64 rb_bytes = (UINT64)store_words * word_stride;
        auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
        auto rb_desc = buffer_desc(rb_bytes);
        ComPtr<ID3D12Resource> readback_buf;
        hr = device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                             D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                             IID_PPV_ARGS(&readback_buf));
        if (FAILED(hr)) { set_detail(out, "CreateCommittedResource(readback)", hr); return -34; }

        D3D12_RESOURCE_BARRIER tb = {};
        tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        tb.Transition.pResource = cluster_store_buf;
        tb.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        tb.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &tb);
        cmd->CopyBufferRegion(readback_buf.Get(), 0, cluster_store_buf, 0, rb_bytes);
        D3D12_RESOURCE_BARRIER tb2 = tb;
        tb2.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb2.Transition.StateAfter = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        cmd->ResourceBarrier(1, &tb2);

        hr = cmd->Close();
        if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
        ID3D12CommandList* lists[] = {cmd.Get()};
        queue->ExecuteCommandLists(1, lists);
        UINT64 submit_value = ++next_fence_value;
        if (FAILED(queue->Signal(fence.Get(), submit_value))) {
            set_detail_msg(out, "Signal fence");
            return -36;
        }
        allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value =
            submit_value;

        if (fence->GetCompletedValue() < submit_value) {
            if (FAILED(fence->SetEventOnCompletion(submit_value, fence_event))) {
                set_detail_msg(out, "SetEventOnCompletion");
                return -37;
            }
            WaitForSingleObject(fence_event, INFINITE);
        }
        out->fence_completed_value = fence->GetCompletedValue();
        if (out->fence_completed_value < submit_value) {
            set_detail_msg(out, "fence did not reach completion");
            return -38;
        }

        uint8_t* mapped = nullptr;
        D3D12_RANGE rng = {0, (SIZE_T)rb_bytes};
        if (FAILED(readback_buf->Map(0, &rng, reinterpret_cast<void**>(&mapped)))) {
            set_detail_msg(out, "Map readback");
            return -39;
        }
        uint32_t checksum = 2166136261u;
        float first = 0.0f;
        if (rb_bytes >= sizeof(float)) {
            std::memcpy(&first, mapped, sizeof(float));
        }
        for (UINT64 i = 0; i < rb_bytes; ++i) {
            checksum ^= mapped[i];
            checksum *= 16777619u;
        }
        readback_buf->Unmap(0, nullptr);

        out->dispatch_x = gx;
        out->dispatch_y = gy;
        out->dispatch_z = 1;
        out->dst_width = cluster_store_bytes;
        out->dst_height = 1;
        out->readback_checksum = checksum;
        out->dst_first_value = first;
        out->dxil_signed = k->dxil_signed ? 1 : 0;
        std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
        std::printf("RXGD_BRIDGE_REC: dispatch=%u,%u,1 fence=%llu dst_bytes=%u dst_first=%g "
                    "checksum=0x%08x dxil_signed=%s\n",
                    gx, gy, (unsigned long long)out->fence_completed_value,
                    cluster_store_bytes, first, checksum,
                    out->dxil_signed ? "yes" : "no");
        std::fflush(stdout);
        return 0;
    }

    // ── Wave 4 bridge structured-buffer view helpers (GRX-015/016/018) ───────
    void create_structured_srv(ID3D12Resource* res, UINT bytes, UINT stride, UINT slot) {
        D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
        srv.Format = DXGI_FORMAT_UNKNOWN;
        srv.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
        srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
        srv.Buffer.FirstElement = 0;
        srv.Buffer.NumElements = (std::max)(bytes / stride, 1u);
        srv.Buffer.StructureByteStride = stride;
        srv.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
        device->CreateShaderResourceView(res, &srv, cpu_handle(slot));
    }
    void create_structured_uav(ID3D12Resource* res, UINT bytes, UINT stride, UINT slot) {
        D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
        uav.Format = DXGI_FORMAT_UNKNOWN;
        uav.ViewDimension = D3D12_UAV_DIMENSION_BUFFER;
        uav.Buffer.FirstElement = 0;
        uav.Buffer.NumElements = (std::max)(bytes / stride, 1u);
        uav.Buffer.StructureByteStride = stride;
        uav.Buffer.CounterOffsetInBytes = 0;
        uav.Buffer.Flags = D3D12_BUFFER_UAV_FLAG_NONE;
        device->CreateUnorderedAccessView(res, nullptr, &uav, cpu_handle(slot));
    }
    // Test-only structured-buffer readback: transition `buf` UNORDERED_ACCESS ->
    // COPY_SOURCE, copy `bytes` into a fresh READBACK buffer, restore state,
    // submit+wait, FNV-1a checksum the bytes, and fill `out`. Returns the
    // checksum; `first_out` receives the first float. Assumes the command list
    // is open with the dispatch(es) already recorded.
    int finish_buffer_readback(uint32_t pass_id, ID3D12Resource* buf, UINT bytes,
                               UINT gx, UINT gy, bool readback, RxgdRecordResult* out,
                               ID3D12PipelineState* /*unused*/) {
        (void)pass_id;
        if (!readback) {
            HRESULT hr = cmd->Close();
            if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
            ID3D12CommandList* plists[] = {cmd.Get()};
            queue->ExecuteCommandLists(1, plists);
            UINT64 pval = ++next_fence_value;
            if (FAILED(queue->Signal(fence.Get(), pval))) {
                set_detail_msg(out, "Signal fence");
                return -36;
            }
            allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value =
                pval;
            out->fence_completed_value = pval;
            out->dispatch_x = gx;
            out->dispatch_y = gy;
            out->dispatch_z = 1;
            out->dst_width = bytes;
            out->dst_height = 1;
            out->readback_checksum = 0;
            out->dst_first_value = 0.0f;
            std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
            return 0;
        }
        const UINT64 rb_bytes = bytes;
        auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
        auto rb_desc = buffer_desc(rb_bytes);
        ComPtr<ID3D12Resource> readback_buf;
        HRESULT hr = device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                                     D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                                     IID_PPV_ARGS(&readback_buf));
        if (FAILED(hr)) { set_detail(out, "CreateCommittedResource(readback)", hr); return -34; }
        D3D12_RESOURCE_BARRIER tb = {};
        tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        tb.Transition.pResource = buf;
        tb.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        tb.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &tb);
        cmd->CopyBufferRegion(readback_buf.Get(), 0, buf, 0, rb_bytes);
        D3D12_RESOURCE_BARRIER tb2 = tb;
        tb2.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb2.Transition.StateAfter = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        cmd->ResourceBarrier(1, &tb2);
        hr = cmd->Close();
        if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
        ID3D12CommandList* lists[] = {cmd.Get()};
        queue->ExecuteCommandLists(1, lists);
        UINT64 submit_value = ++next_fence_value;
        if (FAILED(queue->Signal(fence.Get(), submit_value))) {
            set_detail_msg(out, "Signal fence");
            return -36;
        }
        allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value =
            submit_value;
        if (fence->GetCompletedValue() < submit_value) {
            if (FAILED(fence->SetEventOnCompletion(submit_value, fence_event))) {
                set_detail_msg(out, "SetEventOnCompletion");
                return -37;
            }
            WaitForSingleObject(fence_event, INFINITE);
        }
        out->fence_completed_value = fence->GetCompletedValue();
        if (out->fence_completed_value < submit_value) {
            set_detail_msg(out, "fence did not reach completion");
            return -38;
        }
        uint8_t* mapped = nullptr;
        D3D12_RANGE rng = {0, (SIZE_T)rb_bytes};
        if (FAILED(readback_buf->Map(0, &rng, reinterpret_cast<void**>(&mapped)))) {
            set_detail_msg(out, "Map readback");
            return -39;
        }
        uint32_t checksum = 2166136261u;
        float first = 0.0f;
        if (rb_bytes >= sizeof(float)) std::memcpy(&first, mapped, sizeof(float));
        for (UINT64 i = 0; i < rb_bytes; ++i) { checksum ^= mapped[i]; checksum *= 16777619u; }
        readback_buf->Unmap(0, nullptr);
        out->dispatch_x = gx;
        out->dispatch_y = gy;
        out->dispatch_z = 1;
        out->dst_width = bytes;
        out->dst_height = 1;
        out->readback_checksum = checksum;
        out->dst_first_value = first;
        std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
        std::printf("RXGD_BRIDGE_REC: dispatch=%u,%u,1 fence=%llu dst_bytes=%u dst_first=%g "
                    "checksum=0x%08x dxil_signed=%s\n",
                    gx, gy, (unsigned long long)out->fence_completed_value, bytes, first,
                    checksum, out->dxil_signed ? "yes" : "no");
        std::fflush(stdout);
        return 0;
    }

    // GRX-015 gpu_culling: StructuredBuffer SRV t0 (src_transforms) +
    // RWStructuredBuffer UAV u0 (dst_commands) + RWStructuredBuffer UAV u1
    // (dst_visibility) + 144-byte (36-dword) b0, dispatch ceil(instance_count /
    // 64) where instance_count is b0 dword 24. Test-only readback of dst_commands.
    int record_gpu_culling(uint32_t pass_id, const uint8_t* dxil, size_t dxil_len,
                           const uint8_t* rts0, size_t rts0_len,
                           ID3D12Resource* transforms, ID3D12Resource* commands,
                           ID3D12Resource* visibility, const uint8_t* push_constants,
                           uint32_t transforms_bytes, uint32_t commands_bytes,
                           uint32_t visibility_bytes, bool readback, RxgdRecordResult* out) {
        if (!ensure_initialized(out)) return -31;
        CachedKernel* k = get_or_create_kernel(dxil, dxil_len, rts0, rts0_len, out);
        if (!k) return -32;
        if (!transforms || !commands || !visibility) {
            set_detail_msg(out, "null gpu_culling buffer handle");
            return -41;
        }
        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) { set_detail(out, "command list Reset", hr); return -33; }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);
        UINT base = reserve_descriptors(3);
        create_structured_srv(transforms, transforms_bytes, 4u, base + 0);
        create_structured_uav(commands, commands_bytes, 4u, base + 1);
        create_structured_uav(visibility, visibility_bytes, 4u, base + 2);
        cmd->SetComputeRootSignature(k->root.Get());
        cmd->SetPipelineState(k->pso.Get());
        uint32_t rc[36];
        std::memcpy(rc, push_constants, 144);
        cmd->SetComputeRoot32BitConstants(0, 36, rc, 0);
        cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
        const uint32_t instance_count = rc[24];
        const UINT gx = (std::max)((instance_count + 63u) / 64u, 1u);
        cmd->Dispatch(gx, 1, 1);
        out->dxil_signed = k->dxil_signed ? 1 : 0;
        return finish_buffer_readback(pass_id, commands, commands_bytes, gx, 1u, readback, out,
                                      nullptr);
    }

    // GRX-018 indirect_args: paired write + validate kernels sharing one root
    // signature over StructuredBuffer SRV t0 (src_survivor_counts) +
    // RWStructuredBuffer UAV u0 (dst_command_buffer) + RWStructuredBuffer UAV u1
    // (dst_validation) + 176-byte (44-dword) b0. Records write -> UAV barrier on
    // dst_command_buffer -> validate in one command list; test-only readback of
    // dst_validation.
    int record_indirect_args(uint32_t pass_id, const uint8_t* write_dxil, size_t write_dxil_len,
                             const uint8_t* validate_dxil, size_t validate_dxil_len,
                             const uint8_t* rts0, size_t rts0_len,
                             ID3D12Resource* survivor_counts, ID3D12Resource* command_buffer,
                             ID3D12Resource* validation, const uint8_t* push_constants,
                             uint32_t survivor_bytes, uint32_t command_bytes,
                             uint32_t validation_bytes, bool readback, RxgdRecordResult* out) {
        if (!ensure_initialized(out)) return -31;
        CachedKernel* k_write = get_or_create_kernel(write_dxil, write_dxil_len, rts0, rts0_len, out);
        if (!k_write) return -32;
        CachedKernel* k_validate =
            get_or_create_kernel(validate_dxil, validate_dxil_len, rts0, rts0_len, out);
        if (!k_validate) return -32;
        if (!survivor_counts || !command_buffer || !validation) {
            set_detail_msg(out, "null indirect_args buffer handle");
            return -41;
        }
        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) { set_detail(out, "command list Reset", hr); return -33; }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);
        uint32_t rc[44];
        std::memcpy(rc, push_constants, 176);
        // Write kernel.
        {
            UINT base = reserve_descriptors(3);
            create_structured_srv(survivor_counts, survivor_bytes, 4u, base + 0);
            create_structured_uav(command_buffer, command_bytes, 4u, base + 1);
            create_structured_uav(validation, validation_bytes, 4u, base + 2);
            cmd->SetComputeRootSignature(k_write->root.Get());
            cmd->SetPipelineState(k_write->pso.Get());
            cmd->SetComputeRoot32BitConstants(0, 44, rc, 0);
            cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
            cmd->Dispatch(1, 1, 1);
        }
        // Ordering: the validate kernel reads the command buffer the write
        // kernel produced.
        D3D12_RESOURCE_BARRIER ub = {};
        ub.Type = D3D12_RESOURCE_BARRIER_TYPE_UAV;
        ub.UAV.pResource = command_buffer;
        cmd->ResourceBarrier(1, &ub);
        // Validate kernel (same root signature / descriptor shape).
        {
            UINT base = reserve_descriptors(3);
            create_structured_srv(survivor_counts, survivor_bytes, 4u, base + 0);
            create_structured_uav(command_buffer, command_bytes, 4u, base + 1);
            create_structured_uav(validation, validation_bytes, 4u, base + 2);
            cmd->SetComputeRootSignature(k_validate->root.Get());
            cmd->SetPipelineState(k_validate->pso.Get());
            cmd->SetComputeRoot32BitConstants(0, 44, rc, 0);
            cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
            cmd->Dispatch(1, 1, 1);
        }
        out->dxil_signed = (k_write->dxil_signed && k_validate->dxil_signed) ? 1 : 0;
        return finish_buffer_readback(pass_id, validation, validation_bytes, 1u, 1u, readback, out,
                                      nullptr);
    }

    // GRX-016 instance_compaction: three kernels (scan_local -> UAV barrier ->
    // scan_groups -> UAV barrier -> scatter) over a flat 7-buffer surface,
    // 32-byte (8-dword) b0. resources[] order = [visibility_mask, src_transforms,
    // local_prefix, group_totals, group_offsets, survivor_count, dst_transforms].
    // Test-only readback of dst_transforms (resource 6).
    int record_instance_compaction(uint32_t pass_id, const uint8_t* scan_local_dxil,
                                   size_t scan_local_len, const uint8_t* scan_groups_dxil,
                                   size_t scan_groups_len, const uint8_t* scatter_dxil,
                                   size_t scatter_len, const uint8_t* scan_rts0, size_t scan_rts0_len,
                                   const uint8_t* scatter_rts0, size_t scatter_rts0_len,
                                   void* const* resources, const uint32_t* resource_bytes,
                                   const uint8_t* push_constants, bool readback,
                                   RxgdRecordResult* out) {
        if (!ensure_initialized(out)) return -31;
        CachedKernel* k_local =
            get_or_create_kernel(scan_local_dxil, scan_local_len, scan_rts0, scan_rts0_len, out);
        if (!k_local) return -32;
        CachedKernel* k_groups =
            get_or_create_kernel(scan_groups_dxil, scan_groups_len, scan_rts0, scan_rts0_len, out);
        if (!k_groups) return -32;
        CachedKernel* k_scatter =
            get_or_create_kernel(scatter_dxil, scatter_len, scatter_rts0, scatter_rts0_len, out);
        if (!k_scatter) return -32;
        ID3D12Resource* res[7];
        for (int i = 0; i < 7; ++i) {
            res[i] = reinterpret_cast<ID3D12Resource*>(resources[i]);
            if (!res[i]) { set_detail_msg(out, "null instance_compaction buffer handle"); return -41; }
        }
        // Per-resource stride (u32 vs uint4).
        const UINT stride[7] = {4u, 16u, 4u, 4u, 4u, 4u, 16u};
        uint32_t rc[8];
        std::memcpy(rc, push_constants, 32);
        const uint32_t total_instances = rc[0];
        const UINT groups = (std::max)((total_instances + 255u) / 256u, 1u);

        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) { set_detail(out, "command list Reset", hr); return -33; }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);

        // Logical state tracking for the intermediates re-bound as SRVs.
        D3D12_RESOURCE_STATES state[7];
        state[0] = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE; // visibility_mask
        state[1] = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE; // src_transforms
        for (int i = 2; i < 7; ++i) state[i] = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        auto transition = [&](int idx, D3D12_RESOURCE_STATES after) {
            if (state[idx] == after) return;
            D3D12_RESOURCE_BARRIER b = {};
            b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
            b.Transition.pResource = res[idx];
            b.Transition.StateBefore = state[idx];
            b.Transition.StateAfter = after;
            b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
            cmd->ResourceBarrier(1, &b);
            state[idx] = after;
        };
        auto uav_barrier = [&](int idx) {
            D3D12_RESOURCE_BARRIER b = {};
            b.Type = D3D12_RESOURCE_BARRIER_TYPE_UAV;
            b.UAV.pResource = res[idx];
            cmd->ResourceBarrier(1, &b);
        };

        // D1 scan_local: [SRV vis(0), UAV local_prefix(2), UAV group_totals(3)].
        {
            UINT base = reserve_descriptors(3);
            create_structured_srv(res[0], resource_bytes[0], stride[0], base + 0);
            create_structured_uav(res[2], resource_bytes[2], stride[2], base + 1);
            create_structured_uav(res[3], resource_bytes[3], stride[3], base + 2);
            cmd->SetComputeRootSignature(k_local->root.Get());
            cmd->SetPipelineState(k_local->pso.Get());
            cmd->SetComputeRoot32BitConstants(0, 8, rc, 0);
            cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
            cmd->Dispatch(groups, 1, 1);
        }
        uav_barrier(2);
        uav_barrier(3);
        transition(3, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE); // group_totals -> SRV
        // D2 scan_groups: [SRV group_totals(3), UAV group_offsets(4), UAV survivor_count(5)].
        {
            UINT base = reserve_descriptors(3);
            create_structured_srv(res[3], resource_bytes[3], stride[3], base + 0);
            create_structured_uav(res[4], resource_bytes[4], stride[4], base + 1);
            create_structured_uav(res[5], resource_bytes[5], stride[5], base + 2);
            cmd->SetComputeRootSignature(k_groups->root.Get());
            cmd->SetPipelineState(k_groups->pso.Get());
            cmd->SetComputeRoot32BitConstants(0, 8, rc, 0);
            cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
            cmd->Dispatch(1, 1, 1);
        }
        uav_barrier(4);
        transition(2, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE); // local_prefix -> SRV
        transition(4, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE); // group_offsets -> SRV
        // D3 scatter: [SRV vis(0), SRV src(1), SRV local_prefix(2), SRV group_offsets(4),
        // UAV dst_transforms(6)].
        {
            UINT base = reserve_descriptors(5);
            create_structured_srv(res[0], resource_bytes[0], stride[0], base + 0);
            create_structured_srv(res[1], resource_bytes[1], stride[1], base + 1);
            create_structured_srv(res[2], resource_bytes[2], stride[2], base + 2);
            create_structured_srv(res[4], resource_bytes[4], stride[4], base + 3);
            create_structured_uav(res[6], resource_bytes[6], stride[6], base + 4);
            cmd->SetComputeRootSignature(k_scatter->root.Get());
            cmd->SetPipelineState(k_scatter->pso.Get());
            cmd->SetComputeRoot32BitConstants(0, 8, rc, 0);
            cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
            cmd->Dispatch(groups, 1, 1);
        }
        out->dxil_signed =
            (k_local->dxil_signed && k_groups->dxil_signed && k_scatter->dxil_signed) ? 1 : 0;
        return finish_buffer_readback(pass_id, res[6], resource_bytes[6], groups, 1u, readback, out,
                                      nullptr);
    }

    // GRX-019 fused_post_chain: 3 Texture2D SRVs (t0 src_color, t1 lum_source,
    // t2 prev_luminance) + 2 RWTexture2D UAVs (u0 dst_color, u1 dst_luminance) +
    // 64-byte (16-dword) b0. Dispatch ceil(source_width / 8) x
    // ceil(source_height / 8). Test-only readback of dst_color (u0).
    int record_fused_post_chain(uint32_t pass_id, const uint8_t* dxil, size_t dxil_len,
                                const uint8_t* rts0, size_t rts0_len,
                                ID3D12Resource* src_color, ID3D12Resource* lum_source,
                                ID3D12Resource* prev_luminance, ID3D12Resource* dst_color,
                                ID3D12Resource* dst_luminance, const uint8_t* push_constants,
                                uint32_t width, uint32_t height, bool readback,
                                RxgdRecordResult* out) {
        (void)pass_id;
        if (!ensure_initialized(out)) return -31;
        CachedKernel* k = get_or_create_kernel(dxil, dxil_len, rts0, rts0_len, out);
        if (!k) return -32;
        ID3D12Resource* all[5] = {src_color, lum_source, prev_luminance, dst_color, dst_luminance};
        for (int i = 0; i < 5; ++i) {
            if (!all[i]) { set_detail_msg(out, "null fused_post_chain resource handle"); return -41; }
            const DXGI_FORMAT res_fmt = all[i]->GetDesc().Format;
            if (format_is_typeless(typed_view_format(res_fmt))) {
                std::snprintf(out->error_detail, sizeof(out->error_detail),
                              "bound resource carries an unmapped typeless format "
                              "(slot=%d resource_format=%d)", i, (int)res_fmt);
                return -40;
            }
        }
        ID3D12CommandAllocator* alloc = acquire_allocator();
        HRESULT hr = cmd->Reset(alloc, nullptr);
        if (FAILED(hr)) { set_detail(out, "command list Reset", hr); return -33; }
        ID3D12DescriptorHeap* heaps[] = {heap.Get()};
        cmd->SetDescriptorHeaps(1, heaps);
        UINT base = reserve_descriptors(5);
        UINT slot = base;
        for (int i = 0; i < 3; ++i) {
            ID3D12Resource* srv_res = all[i];
            D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
            srv.Format = typed_view_format(srv_res->GetDesc().Format);
            srv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
            srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
            srv.Texture2D.MipLevels = 1;
            device->CreateShaderResourceView(srv_res, &srv, cpu_handle(slot++));
        }
        for (int i = 3; i < 5; ++i) {
            D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
            uav.Format = typed_view_format(all[i]->GetDesc().Format);
            uav.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
            device->CreateUnorderedAccessView(all[i], nullptr, &uav, cpu_handle(slot++));
        }
        cmd->SetComputeRootSignature(k->root.Get());
        cmd->SetPipelineState(k->pso.Get());
        uint32_t rc[16];
        std::memcpy(rc, push_constants, 64);
        cmd->SetComputeRoot32BitConstants(0, 16, rc, 0);
        cmd->SetComputeRootDescriptorTable(1, gpu_handle(base));
        const UINT gx = (std::max)((width + 7u) / 8u, 1u);
        const UINT gy = (std::max)((height + 7u) / 8u, 1u);
        cmd->Dispatch(gx, gy, 1);
        out->dxil_signed = k->dxil_signed ? 1 : 0;

        if (!readback) {
            hr = cmd->Close();
            if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
            ID3D12CommandList* plists[] = {cmd.Get()};
            queue->ExecuteCommandLists(1, plists);
            UINT64 pval = ++next_fence_value;
            if (FAILED(queue->Signal(fence.Get(), pval))) { set_detail_msg(out, "Signal fence"); return -36; }
            allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value = pval;
            out->fence_completed_value = pval;
            out->dispatch_x = gx; out->dispatch_y = gy; out->dispatch_z = 1;
            out->dst_width = width; out->dst_height = height;
            out->readback_checksum = 0; out->dst_first_value = 0.0f;
            std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
            return 0;
        }
        // Test-only readback of dst_color via a placed-footprint texture copy.
        D3D12_RESOURCE_DESC dsc = dst_color->GetDesc();
        D3D12_RESOURCE_DESC fp = {};
        fp.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
        fp.Width = dsc.Width; fp.Height = dsc.Height; fp.DepthOrArraySize = 1; fp.MipLevels = 1;
        fp.Format = typed_view_format(dsc.Format); fp.SampleDesc.Count = 1;
        fp.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
        DXGI_FORMAT rb_format = fp.Format;
        D3D12_PLACED_SUBRESOURCE_FOOTPRINT dfp = {};
        UINT drows = 0; UINT64 drow_size = 0, dtotal = 0;
        device->GetCopyableFootprints(&fp, 0, 1, 0, &dfp, &drows, &drow_size, &dtotal);
        if (dtotal == 0 || dtotal == UINT64_MAX) {
            set_detail_msg(out, "GetCopyableFootprints returned an invalid total size");
            return -34;
        }
        auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
        auto rb_desc = buffer_desc(dtotal);
        ComPtr<ID3D12Resource> readback_buf;
        hr = device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                             D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                             IID_PPV_ARGS(&readback_buf));
        if (FAILED(hr)) { set_detail(out, "CreateCommittedResource(readback)", hr); return -34; }
        D3D12_RESOURCE_BARRIER tb = {};
        tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        tb.Transition.pResource = dst_color;
        tb.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        tb.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &tb);
        D3D12_TEXTURE_COPY_LOCATION cdst = {};
        cdst.pResource = readback_buf.Get();
        cdst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
        cdst.PlacedFootprint = dfp;
        D3D12_TEXTURE_COPY_LOCATION csrc = {};
        csrc.pResource = dst_color;
        csrc.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        csrc.SubresourceIndex = 0;
        cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
        D3D12_RESOURCE_BARRIER tb2 = tb;
        tb2.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_SOURCE;
        tb2.Transition.StateAfter = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        cmd->ResourceBarrier(1, &tb2);
        hr = cmd->Close();
        if (FAILED(hr)) { set_detail(out, "Close command list", hr); return -35; }
        ID3D12CommandList* lists[] = {cmd.Get()};
        queue->ExecuteCommandLists(1, lists);
        UINT64 submit_value = ++next_fence_value;
        if (FAILED(queue->Signal(fence.Get(), submit_value))) { set_detail_msg(out, "Signal fence"); return -36; }
        allocators[(alloc_cursor + allocators.size() - 1) % allocators.size()].fence_value = submit_value;
        if (fence->GetCompletedValue() < submit_value) {
            if (FAILED(fence->SetEventOnCompletion(submit_value, fence_event))) {
                set_detail_msg(out, "SetEventOnCompletion");
                return -37;
            }
            WaitForSingleObject(fence_event, INFINITE);
        }
        out->fence_completed_value = fence->GetCompletedValue();
        uint8_t* mapped = nullptr;
        D3D12_RANGE range = {0, (SIZE_T)dtotal};
        if (FAILED(readback_buf->Map(0, &range, reinterpret_cast<void**>(&mapped)))) {
            set_detail_msg(out, "Map readback");
            return -39;
        }
        UINT bpp = dxgi_format_bytes(rb_format);
        if (bpp == 0) bpp = 1;
        const UINT row_pitch = dfp.Footprint.RowPitch;
        const UINT rows = (std::min)(height, dfp.Footprint.Height);
        const UINT cols = (std::min)(width, row_pitch / bpp);
        const uint8_t* const map_end = mapped + (SIZE_T)dtotal;
        const UINT sample_bytes = (std::min)(bpp, 4u);
        uint32_t checksum = 2166136261u;
        float first = 0.0f;
        bool got_first = false;
        for (UINT y = 0; y < rows; ++y) {
            const uint8_t* rowp = mapped + dfp.Offset + (SIZE_T)y * row_pitch;
            for (UINT x = 0; x < cols; ++x) {
                const uint8_t* px = rowp + (SIZE_T)x * bpp;
                if (px + sample_bytes > map_end) break;
                if (!got_first) { std::memcpy(&first, px, sample_bytes); got_first = true; }
                for (UINT b = 0; b < sample_bytes; ++b) { checksum ^= px[b]; checksum *= 16777619u; }
            }
        }
        readback_buf->Unmap(0, nullptr);
        out->dispatch_x = gx; out->dispatch_y = gy; out->dispatch_z = 1;
        out->dst_width = width; out->dst_height = height;
        out->readback_checksum = checksum; out->dst_first_value = first;
        std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
        std::printf("RXGD_BRIDGE_REC: dispatch=%u,%u,1 fence=%llu dst=%ux%u dst_first=%g "
                    "checksum=0x%08x dxil_signed=%s\n",
                    gx, gy, (unsigned long long)out->fence_completed_value, width, height,
                    first, checksum, out->dxil_signed ? "yes" : "no");
        std::fflush(stdout);
        return 0;
    }

    // GRX-021: prewarm a kernel identity (build + cache its root signature + PSO)
    // so the first real dispatch does not pay the lazy create cost. Returns true
    // when the kernel is cached (or already was), false on any create failure.
    // Best-effort: a failure never fails the session (the caller degrades to the
    // lazy path).
    bool prewarm_kernel(const uint8_t* dxil, size_t dxil_len, const uint8_t* rts0,
                        size_t rts0_len) {
        if (!dxil || dxil_len == 0 || !rts0 || rts0_len == 0) return false;
        RxgdRecordResult scratch = {};
        return get_or_create_kernel(dxil, dxil_len, rts0, rts0_len, &scratch) != nullptr;
    }
};

// Per-(device,queue) session registry. The test-only recording feature drives
// one bridge session at a time; keying by (device,queue) is sufficient and lets
// the historical single-dispatch entry (which carries no session handle) reuse
// the same cached D3D12 state as the explicit multi-level entry.
static std::mutex g_registry_mutex;
static std::map<std::pair<void*, void*>, ShimSession*> g_sessions;

static ShimSession* get_or_create_session(void* device, void* queue) {
    std::lock_guard<std::mutex> lock(g_registry_mutex);
    auto key = std::make_pair(device, queue);
    auto it = g_sessions.find(key);
    if (it != g_sessions.end()) return it->second;
    ShimSession* s = new ShimSession(reinterpret_cast<ID3D12Device*>(device),
                                     reinterpret_cast<ID3D12CommandQueue*>(queue));
    g_sessions.emplace(key, s);
    return s;
}

static void close_session(void* device, void* queue) {
    std::lock_guard<std::mutex> lock(g_registry_mutex);
    auto key = std::make_pair(device, queue);
    auto it = g_sessions.find(key);
    if (it == g_sessions.end()) return;
    it->second->print_summary();
    delete it->second;
    g_sessions.erase(it);
}

// ── C ABI ──────────────────────────────────────────────────────────────────

extern "C" __declspec(dllexport) uint32_t rxgd_luminance_record_shim_abi_version(void) {
    return kShimAbiVersion;
}

// Flush + tear down the cached session for (device,queue). Prints one
// RXGD_SUMMARY line per pass the session recorded. Driven from the Rust
// rxgd_destroy_session under the d3d12-recording-shim feature. Safe to call for
// a (device,queue) that never recorded (no-op).
extern "C" __declspec(dllexport) void rxgd_luminance_record_shim_session_close(
    uint32_t abi_version, void* device_ptr, void* queue_ptr) {
    if (abi_version != kShimAbiVersion) return;
    close_session(device_ptr, queue_ptr);
}

// Historical single-level 2-resource (SRV t0 src, UAV u0 dst) recording entry.
// `readback` selects the test/recording instrumented mode (readback + fence
// wait + checksum + RXGD_BRIDGE_REC marker, readback != 0) vs the production
// mode (submit only; zero readback / fence-wait / checksum / stdout marker,
// readback == 0). Returns 0 on success, negative on failure. Never accepts null
// handles and never fakes a dispatch.
extern "C" __declspec(dllexport) int32_t rxgd_luminance_record_dispatch(
    uint32_t abi_version,
    uint32_t pass_id,
    void* device_ptr,
    void* queue_ptr,
    const uint8_t* dxil_bytes,
    size_t dxil_len,
    const uint8_t* rts0_bytes,
    size_t rts0_len,
    void* src_resource_ptr,
    void* dst_resource_ptr,
    const uint8_t* push_constants,
    size_t push_constant_len,
    uint32_t src_w,
    uint32_t src_h,
    uint32_t dst_w,
    uint32_t dst_h,
    uint32_t readback,
    RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));

    if (abi_version != kShimAbiVersion) {
        set_detail_msg(out, "shim abi version mismatch");
        return -2;
    }
    if (!device_ptr || !queue_ptr || !src_resource_ptr || !dst_resource_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!dxil_bytes || dxil_len == 0 || !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 28) {
        set_detail_msg(out, "push constant block is not 28 bytes");
        return -5;
    }

    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);

    RxgdShimKernel kernel = {};
    kernel.dxil = dxil_bytes;
    kernel.dxil_len = dxil_len;
    kernel.rts0 = rts0_bytes;
    kernel.rts0_len = rts0_len;
    kernel.binding_count = 2;  // SRV t0 (src) + UAV u0 (dst)

    RxgdShimResource resources[2] = {};
    resources[0].resource = src_resource_ptr;
    resources[1].resource = dst_resource_ptr;

    RxgdShimLevel level = {};
    level.kernel_index = 0;
    level.srv_index = 0;
    level.uav_index = 1;
    level.prev_index = 0;  // unused for a 2-binding kernel
    level.dispatch_x = std::max<UINT>((src_w + 7) / 8, 1u);
    level.dispatch_y = std::max<UINT>((src_h + 7) / 8, 1u);
    level.dispatch_z = 1;
    level.dst_width = std::max<UINT>(dst_w, 1u);
    level.dst_height = std::max<UINT>(dst_h, 1u);
    std::memcpy(level.push_constants, push_constants, 28);

    int rc = session->record_levels(pass_id, &kernel, 1, resources, 2, &level, 1,
                                    /*readback=*/readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// Multi-kernel / multi-resource / multi-level pyramid recording entry.
// Records `level_count` dispatches (reduce chain + final WRITE_LUMINANCE) with
// inter-level barriers in ONE command list and ONE submit. `readback` selects
// test-only readback + fence wait + RXGD_BRIDGE_REC marker (non-zero) vs the
// production no-wait path (zero). Defined for the later enablement smoke slices;
// no real-GPU test in the cargo-only slice drives it.
extern "C" __declspec(dllexport) int32_t rxgd_luminance_record_levels(
    uint32_t abi_version,
    uint32_t pass_id,
    void* device_ptr,
    void* queue_ptr,
    const RxgdShimKernel* kernels,
    uint32_t kernel_count,
    const RxgdShimResource* resources,
    uint32_t resource_count,
    const RxgdShimLevel* levels,
    uint32_t level_count,
    uint32_t readback,
    RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));

    if (abi_version != kShimAbiVersion) {
        set_detail_msg(out, "shim abi version mismatch");
        return -2;
    }
    if (!device_ptr || !queue_ptr) {
        set_detail_msg(out, "null device/queue handle");
        return -3;
    }
    if (!kernels || kernel_count == 0 || !resources || resource_count == 0 ||
        !levels || level_count == 0) {
        set_detail_msg(out, "empty kernels/resources/levels");
        return -4;
    }
    // Fail closed on any null resource / kernel byte range, and on any level
    // index out of range: never dispatch against an undefined binding.
    for (uint32_t i = 0; i < resource_count; ++i) {
        if (!resources[i].resource) {
            set_detail_msg(out, "null resource handle in levels job");
            return -5;
        }
    }
    for (uint32_t i = 0; i < kernel_count; ++i) {
        if (!kernels[i].dxil || kernels[i].dxil_len == 0 || !kernels[i].rts0 ||
            kernels[i].rts0_len == 0 ||
            (kernels[i].binding_count != 2 && kernels[i].binding_count != 3)) {
            set_detail_msg(out, "invalid kernel byte range / binding_count");
            return -6;
        }
    }
    for (uint32_t i = 0; i < level_count; ++i) {
        const RxgdShimLevel& lv = levels[i];
        if (lv.kernel_index >= kernel_count || lv.srv_index >= resource_count ||
            lv.uav_index >= resource_count) {
            set_detail_msg(out, "level references out-of-range kernel/resource index");
            return -7;
        }
        if (kernels[lv.kernel_index].binding_count == 3 &&
            lv.prev_index >= resource_count) {
            set_detail_msg(out, "write level references out-of-range prev index");
            return -8;
        }
    }

    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int rc = session->record_levels(pass_id, kernels, kernel_count, resources,
                                    resource_count, levels, level_count,
                                    readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// GRX-012: 5-SRV (t0..t4) + 1-UAV (u0) single-dispatch recording entry for the
// taa_resolve kernel. `readback` selects the test/recording instrumented mode
// (readback + fence wait + checksum + RXGD_BRIDGE_REC marker) vs the production
// mode (submit only). Returns 0 on success, negative on failure. Never accepts
// null handles and never fakes a dispatch. Additive; the existing 2-resource /
// multi-level entries are untouched.
extern "C" __declspec(dllexport) int32_t rxgd_taa_resolve_record_dispatch(
    uint32_t abi_version,
    uint32_t pass_id,
    void* device_ptr,
    void* queue_ptr,
    const uint8_t* dxil_bytes,
    size_t dxil_len,
    const uint8_t* rts0_bytes,
    size_t rts0_len,
    void* color_ptr,
    void* depth_ptr,
    void* velocity_ptr,
    void* last_velocity_ptr,
    void* history_ptr,
    void* output_ptr,
    const uint8_t* push_constants,
    size_t push_constant_len,
    uint32_t width,
    uint32_t height,
    uint32_t readback,
    RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));

    if (abi_version != kShimAbiVersion) {
        set_detail_msg(out, "shim abi version mismatch");
        return -2;
    }
    if (!device_ptr || !queue_ptr || !color_ptr || !depth_ptr || !velocity_ptr ||
        !last_velocity_ptr || !history_ptr || !output_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!dxil_bytes || dxil_len == 0 || !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 28) {
        set_detail_msg(out, "push constant block is not 28 bytes");
        return -5;
    }

    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    ID3D12Resource* srvs[5] = {
        reinterpret_cast<ID3D12Resource*>(color_ptr),
        reinterpret_cast<ID3D12Resource*>(depth_ptr),
        reinterpret_cast<ID3D12Resource*>(velocity_ptr),
        reinterpret_cast<ID3D12Resource*>(last_velocity_ptr),
        reinterpret_cast<ID3D12Resource*>(history_ptr),
    };
    ID3D12Resource* uav = reinterpret_cast<ID3D12Resource*>(output_ptr);

    int rc = session->record_taa(pass_id, dxil_bytes, dxil_len, rts0_bytes, rts0_len,
                                 srvs, uav, push_constants, width, height,
                                 readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

extern "C" __declspec(dllexport) int32_t rxgd_particles_copy_record_dispatch(
    uint32_t abi_version,
    uint32_t pass_id,
    void* device_ptr,
    void* queue_ptr,
    const uint8_t* dxil_bytes,
    size_t dxil_len,
    const uint8_t* rts0_bytes,
    size_t rts0_len,
    void* src_particles_ptr,
    void* dst_instances_ptr,
    const uint8_t* push_constants,
    size_t push_constant_len,
    uint32_t src_bytes,
    uint32_t dst_bytes,
    uint32_t readback,
    RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));

    if (abi_version != kShimAbiVersion) {
        set_detail_msg(out, "shim abi version mismatch");
        return -2;
    }
    if (!device_ptr || !queue_ptr || !src_particles_ptr || !dst_instances_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!dxil_bytes || dxil_len == 0 || !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 128) {
        set_detail_msg(out, "push constant block is not 128 bytes");
        return -5;
    }

    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int rc = session->record_particles_copy(
        pass_id, dxil_bytes, dxil_len, rts0_bytes, rts0_len,
        reinterpret_cast<ID3D12Resource*>(src_particles_ptr),
        reinterpret_cast<ID3D12Resource*>(dst_instances_ptr),
        push_constants, src_bytes, dst_bytes, readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// GRX-014: 2-SRV (t0 cluster_render uint words, t1 render_elements 80-byte
// RenderElementData) + 1-UAV (u0 cluster_store uint words) single-dispatch
// recording entry for the cluster_store kernel. `readback` selects the
// test/recording instrumented mode (readback + fence wait + checksum +
// RXGD_BRIDGE_REC marker) vs the production mode (submit only). The dispatch
// is ceil(cluster_screen_size / 8)² with cluster_screen_size read from the b0
// dwords 2-3. Returns 0 on success, negative on failure. Never accepts null
// handles and never fakes a dispatch. Additive; the existing 2-resource / taa
// / particles / multi-level entries are untouched.
extern "C" __declspec(dllexport) int32_t rxgd_cluster_store_record_dispatch(
    uint32_t abi_version,
    uint32_t pass_id,
    void* device_ptr,
    void* queue_ptr,
    const uint8_t* dxil_bytes,
    size_t dxil_len,
    const uint8_t* rts0_bytes,
    size_t rts0_len,
    void* cluster_render_ptr,
    void* render_elements_ptr,
    void* cluster_store_ptr,
    const uint8_t* push_constants,
    size_t push_constant_len,
    uint32_t cluster_render_bytes,
    uint32_t render_elements_bytes,
    uint32_t cluster_store_bytes,
    uint32_t readback,
    RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));

    if (abi_version != kShimAbiVersion) {
        set_detail_msg(out, "shim abi version mismatch");
        return -2;
    }
    if (!device_ptr || !queue_ptr || !cluster_render_ptr || !render_elements_ptr ||
        !cluster_store_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!dxil_bytes || dxil_len == 0 || !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 32) {
        set_detail_msg(out, "push constant block is not 32 bytes");
        return -5;
    }

    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int rc = session->record_cluster_store(
        pass_id, dxil_bytes, dxil_len, rts0_bytes, rts0_len,
        reinterpret_cast<ID3D12Resource*>(cluster_render_ptr),
        reinterpret_cast<ID3D12Resource*>(render_elements_ptr),
        reinterpret_cast<ID3D12Resource*>(cluster_store_ptr),
        push_constants, cluster_render_bytes, render_elements_bytes,
        cluster_store_bytes, readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// GRX-015: gpu_culling 1-SRV + 2-UAV structured-buffer single-dispatch entry
// (src_transforms t0 + dst_commands u0 + dst_visibility u1 + 144-byte b0).
// Additive; the existing entries are untouched.
extern "C" __declspec(dllexport) int32_t rxgd_gpu_culling_record_dispatch(
    uint32_t abi_version, uint32_t pass_id, void* device_ptr, void* queue_ptr,
    const uint8_t* dxil_bytes, size_t dxil_len, const uint8_t* rts0_bytes, size_t rts0_len,
    void* transforms_ptr, void* commands_ptr, void* visibility_ptr,
    const uint8_t* push_constants, size_t push_constant_len,
    uint32_t transforms_bytes, uint32_t commands_bytes, uint32_t visibility_bytes,
    uint32_t readback, RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));
    if (abi_version != kShimAbiVersion) { set_detail_msg(out, "shim abi version mismatch"); return -2; }
    if (!device_ptr || !queue_ptr || !transforms_ptr || !commands_ptr || !visibility_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!dxil_bytes || dxil_len == 0 || !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 144) {
        set_detail_msg(out, "push constant block is not 144 bytes");
        return -5;
    }
    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int rc = session->record_gpu_culling(
        pass_id, dxil_bytes, dxil_len, rts0_bytes, rts0_len,
        reinterpret_cast<ID3D12Resource*>(transforms_ptr),
        reinterpret_cast<ID3D12Resource*>(commands_ptr),
        reinterpret_cast<ID3D12Resource*>(visibility_ptr),
        push_constants, transforms_bytes, commands_bytes, visibility_bytes, readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// GRX-018: indirect_args paired write + validate entry over the shared 3-buffer
// descriptor table (src_survivor_counts t0 + dst_command_buffer u0 +
// dst_validation u1 + 176-byte b0). Additive.
extern "C" __declspec(dllexport) int32_t rxgd_indirect_args_record_dispatch(
    uint32_t abi_version, uint32_t pass_id, void* device_ptr, void* queue_ptr,
    const uint8_t* write_dxil, size_t write_dxil_len, const uint8_t* validate_dxil,
    size_t validate_dxil_len, const uint8_t* rts0_bytes, size_t rts0_len,
    void* survivor_ptr, void* command_ptr, void* validation_ptr,
    const uint8_t* push_constants, size_t push_constant_len,
    uint32_t survivor_bytes, uint32_t command_bytes, uint32_t validation_bytes,
    uint32_t readback, RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));
    if (abi_version != kShimAbiVersion) { set_detail_msg(out, "shim abi version mismatch"); return -2; }
    if (!device_ptr || !queue_ptr || !survivor_ptr || !command_ptr || !validation_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!write_dxil || write_dxil_len == 0 || !validate_dxil || validate_dxil_len == 0 ||
        !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 176) {
        set_detail_msg(out, "push constant block is not 176 bytes");
        return -5;
    }
    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int rc = session->record_indirect_args(
        pass_id, write_dxil, write_dxil_len, validate_dxil, validate_dxil_len, rts0_bytes, rts0_len,
        reinterpret_cast<ID3D12Resource*>(survivor_ptr),
        reinterpret_cast<ID3D12Resource*>(command_ptr),
        reinterpret_cast<ID3D12Resource*>(validation_ptr),
        push_constants, survivor_bytes, command_bytes, validation_bytes, readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// GRX-016: instance_compaction three-kernel chain entry. `resources`/
// `resource_bytes` are 7-element arrays in the flat surface order
// [visibility_mask, src_transforms, local_prefix, group_totals, group_offsets,
// survivor_count, dst_transforms]. 32-byte b0. Additive.
extern "C" __declspec(dllexport) int32_t rxgd_instance_compaction_record_dispatch(
    uint32_t abi_version, uint32_t pass_id, void* device_ptr, void* queue_ptr,
    const uint8_t* scan_local_dxil, size_t scan_local_len, const uint8_t* scan_groups_dxil,
    size_t scan_groups_len, const uint8_t* scatter_dxil, size_t scatter_len,
    const uint8_t* scan_rts0, size_t scan_rts0_len, const uint8_t* scatter_rts0,
    size_t scatter_rts0_len, void* const* resources, uint32_t resource_count,
    const uint32_t* resource_bytes, const uint8_t* push_constants, size_t push_constant_len,
    uint32_t readback, RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));
    if (abi_version != kShimAbiVersion) { set_detail_msg(out, "shim abi version mismatch"); return -2; }
    if (!device_ptr || !queue_ptr || !resources || !resource_bytes || resource_count != 7) {
        set_detail_msg(out, "null device/queue/resources or resource_count != 7");
        return -3;
    }
    if (!scan_local_dxil || scan_local_len == 0 || !scan_groups_dxil || scan_groups_len == 0 ||
        !scatter_dxil || scatter_len == 0 || !scan_rts0 || scan_rts0_len == 0 ||
        !scatter_rts0 || scatter_rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 32) {
        set_detail_msg(out, "push constant block is not 32 bytes");
        return -5;
    }
    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int rc = session->record_instance_compaction(
        pass_id, scan_local_dxil, scan_local_len, scan_groups_dxil, scan_groups_len, scatter_dxil,
        scatter_len, scan_rts0, scan_rts0_len, scatter_rts0, scatter_rts0_len, resources,
        resource_bytes, push_constants, readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// GRX-019: fused_post_chain 3-SRV + 2-UAV texture single-dispatch entry
// (src_color t0 + lum_source t1 + prev_luminance t2 + dst_color u0 +
// dst_luminance u1 + 64-byte b0). Additive.
extern "C" __declspec(dllexport) int32_t rxgd_fused_post_chain_record_dispatch(
    uint32_t abi_version, uint32_t pass_id, void* device_ptr, void* queue_ptr,
    const uint8_t* dxil_bytes, size_t dxil_len, const uint8_t* rts0_bytes, size_t rts0_len,
    void* src_color_ptr, void* lum_source_ptr, void* prev_luminance_ptr, void* dst_color_ptr,
    void* dst_luminance_ptr, const uint8_t* push_constants, size_t push_constant_len,
    uint32_t width, uint32_t height, uint32_t readback, RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));
    if (abi_version != kShimAbiVersion) { set_detail_msg(out, "shim abi version mismatch"); return -2; }
    if (!device_ptr || !queue_ptr || !src_color_ptr || !lum_source_ptr || !prev_luminance_ptr ||
        !dst_color_ptr || !dst_luminance_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!dxil_bytes || dxil_len == 0 || !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 64) {
        set_detail_msg(out, "push constant block is not 64 bytes");
        return -5;
    }
    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int rc = session->record_fused_post_chain(
        pass_id, dxil_bytes, dxil_len, rts0_bytes, rts0_len,
        reinterpret_cast<ID3D12Resource*>(src_color_ptr),
        reinterpret_cast<ID3D12Resource*>(lum_source_ptr),
        reinterpret_cast<ID3D12Resource*>(prev_luminance_ptr),
        reinterpret_cast<ID3D12Resource*>(dst_color_ptr),
        reinterpret_cast<ID3D12Resource*>(dst_luminance_ptr),
        push_constants, width, height, readback != 0, out);
    session->note(pass_id, rc == 0);
    return rc;
}

// GRX-021: prewarm a batch of kernel identities for the (device, queue) session
// so the first real dispatch does not pay the lazy root-signature/PSO create
// cost. Returns the number of kernels successfully cached (>= 0), or a negative
// status on an ABI mismatch / null argument. Best-effort by contract: a partial
// or total failure is NOT fatal — the caller (the Rust bridge) never fails the
// session on prewarm failure; the lazy path still builds each kernel on first use.
extern "C" __declspec(dllexport) int32_t rxgd_prewarm_kernels(
    uint32_t abi_version, void* device_ptr, void* queue_ptr, const RxgdShimKernel* kernels,
    uint32_t kernel_count) {
    if (abi_version != kShimAbiVersion) return -2;
    if (!device_ptr || !queue_ptr) return -3;
    if (!kernels || kernel_count == 0) return 0;
    ShimSession* session = get_or_create_session(device_ptr, queue_ptr);
    int32_t warmed = 0;
    for (uint32_t i = 0; i < kernel_count; ++i) {
        const RxgdShimKernel& k = kernels[i];
        if (session->prewarm_kernel(k.dxil, k.dxil_len, k.rts0, k.rts0_len)) {
            warmed += 1;
        }
    }
    return warmed;
}
