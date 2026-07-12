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
#include <utility>
#include <vector>

#ifdef RXGD_HAVE_DXCAPI
#include <dxcapi.h>
#endif

using Microsoft::WRL::ComPtr;

// Shim <-> Rust ABI version (kept in sync with the Rust d3d12_shim module).
// Bumped 1 -> 2 for the Wave 2 v2 execution model (session cache + rings +
// multi-level record entry + summary + explicit session close).
static const uint32_t kShimAbiVersion = 2u;

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
    ComPtr<ID3D12Fence> fence;
    UINT64 next_fence_value = 0;
    HANDLE fence_event = nullptr;
    bool initialized = false;
    std::map<uint32_t, PassCounters> counters;

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

    // Reserve `count` contiguous descriptors from the shader-visible heap,
    // returning the starting index. Wraps around (test-only ring; a big heap
    // keeps in-flight submissions from colliding).
    UINT reserve_descriptors(UINT count) {
        if (heap_cursor + count > kDescriptorHeapCapacity) heap_cursor = 0;
        UINT base = heap_cursor;
        heap_cursor += count;
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
    }

    // Print one machine-readable summary line per pass touched this session.
    void print_summary() {
        for (const auto& kv : counters) {
            std::printf("RXGD_SUMMARY pass=%u recorded=%llu fallback=%llu\n",
                        kv.first, (unsigned long long)kv.second.recorded,
                        (unsigned long long)kv.second.fallback);
        }
        std::fflush(stdout);
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
// Always test-only readback mode. Returns 0 on success, negative on failure.
// Never accepts null handles and never fakes a dispatch.
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
                                    /*readback=*/true, out);
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
