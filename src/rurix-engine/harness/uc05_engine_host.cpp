// uc05_engine_host.cpp — engine_host **v2**(EI1.4,D-EI1-5 / G-EI1-4;RFC-0014 §4.A+§4.B /
// spec/export_c.md + spec/rhi.md;RXS-0250~0255 + RXS-0261)。
//
// **与 v1(engine_host.cpp,G1.3 / MR-0002 / RXS-0149)的关系**:v1 是**手写路**——宿主
// include 手写头 `include/rurix_engine.h`、链接 Rust crate `src/rurix-engine` 产的 cdylib,
// 头↔ABI 一致性由 RXS-0149 冻结守卫保障。v2 是**生成路**——宿主 include **编译器自始生成**
// 的头 `rurix_rhi.h`(RXS-0253)、链接 `.rx` 单源经 `rurixc --emit=dll` 产的 `rurix_rhi.dll`
// (RXS-0252),头↔ABI 一致性由「同一份 C 映射同源产 /EXPORT: 与 .h」结构性保障 + CI 再生成
// 逐字节比对守卫(RXS-0254)。两制共存(RXS-0254 §4.A5):**v1 既有资产逐字节 0-byte**,
// 本文件为新增,不改 v1 的三符号面 / 手写头 / RXS-0149 守卫。
//
// 极性(同 v1):宿主 C++/D3D12 框架是**驱动方**,Rurix 是被调的 compute pass 提供方。宿主
// 建立最小 render-graph 上下文(在与 CUDA device 同 adapter = LUID 匹配的 D3D12 device +
// command queue + fence),把**一整个 Rurix RHI 图**(两 compute pass:fill → scale,含图装配
// 期 hazard 推导与真派发)作为**单个图节点**在宿主帧序内执行——宿主 fence signal → 节点 →
// fence signal/wait,证 Rurix 图在宿主的时间轴上有确定位置。
//
// 宿主只见 C ABI 标量与裸指针:GPU 上下文 / 图 / 资源生命周期全部封闭在 `uc05_run_graph`
// 一次调用内。数值对照 vs **宿主侧独立算的**闭式参考 `n*(n+1)`(与 .rx 侧 `graph::uc05_reference`
// 同公式、不同实现——对照因此非自证)。
//
// 编译(device 段,MSVC + Windows SDK D3D12 + CUDA Toolkit;由 ci/uc05_engine_embed_smoke.py 编排):
//   cl /std:c++17 /EHsc /I <生成头目录> /I "%CUDA_PATH%\include" uc05_engine_host.cpp ^
//      /link rurix_rhi.lib cudart.lib d3d12.lib dxgi.lib /LIBPATH:"%CUDA_PATH%\lib\x64"
// 运行:uc05_engine_host.exe   (退出码 0 = 全部对照通过;非 0 见下方各 return)

#include <cstdint>
#include <cstdio>
#include <cstring>

#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <wrl/client.h>

#include <cuda_runtime.h>

// **编译器生成头**(rurixc --emit=dll,RXS-0253);不手写、不随仓库提交为源——由 CI 于每次
// 运行现场再生成并逐字节比对(RXS-0254)。
#include "rurix_rhi.h"

using Microsoft::WRL::ComPtr;

namespace {

// 宿主侧独立参考:sum_{i=0}^{n-1} 2*(i+1) = n*(n+1)。u32 域内 n <= 65535 无溢出。
uint32_t host_reference(uint32_t n) { return n * (n + 1u); }

bool create_d3d12_on_cuda_adapter(int cuda_device, ComPtr<ID3D12Device>& device,
                                  ComPtr<ID3D12CommandQueue>& queue) {
    cudaDeviceProp prop{};
    if (cudaGetDeviceProperties(&prop, cuda_device) != cudaSuccess) {
        std::fprintf(stderr, "UC05_HOST: cudaGetDeviceProperties failed\n");
        return false;
    }
    ComPtr<IDXGIFactory4> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) {
        std::fprintf(stderr, "UC05_HOST: CreateDXGIFactory2 failed\n");
        return false;
    }
    // 选与 CUDA device LUID 相同的 DXGI adapter(RFC-0001 §4.4 LUID 匹配;同 v1)。
    ComPtr<IDXGIAdapter1> adapter;
    for (UINT i = 0; factory->EnumAdapters1(i, &adapter) != DXGI_ERROR_NOT_FOUND; ++i) {
        DXGI_ADAPTER_DESC1 desc{};
        adapter->GetDesc1(&desc);
        if (std::memcmp(&desc.AdapterLuid, prop.luid, sizeof(desc.AdapterLuid)) == 0) {
            break;
        }
        adapter.Reset();
    }
    if (!adapter) {
        std::fprintf(stderr, "UC05_HOST: no DXGI adapter matches CUDA LUID\n");
        return false;
    }
    if (FAILED(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)))) {
        std::fprintf(stderr, "UC05_HOST: D3D12CreateDevice failed\n");
        return false;
    }
    D3D12_COMMAND_QUEUE_DESC qdesc{};
    qdesc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    if (FAILED(device->CreateCommandQueue(&qdesc, IID_PPV_ARGS(&queue)))) {
        std::fprintf(stderr, "UC05_HOST: CreateCommandQueue failed\n");
        return false;
    }
    return true;
}

// 宿主帧序锚点:在 D3D12 queue 上 signal 一个 fence 值并 CPU 侧等待其完成。Rurix 图节点
// 夹在两个锚点之间执行 —— 证图节点在宿主时间轴上有确定位置(非「另起一条无关时间线」)。
bool queue_fence_barrier(ID3D12Device* device, ID3D12CommandQueue* queue, ComPtr<ID3D12Fence>& fence,
                         UINT64& value, HANDLE event) {
    if (!fence && FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) {
        std::fprintf(stderr, "UC05_HOST: CreateFence failed\n");
        return false;
    }
    ++value;
    if (FAILED(queue->Signal(fence.Get(), value))) {
        std::fprintf(stderr, "UC05_HOST: fence Signal failed\n");
        return false;
    }
    if (fence->GetCompletedValue() < value) {
        if (FAILED(fence->SetEventOnCompletion(value, event))) {
            std::fprintf(stderr, "UC05_HOST: SetEventOnCompletion failed\n");
            return false;
        }
        WaitForSingleObject(event, 10000);
    }
    return fence->GetCompletedValue() >= value;
}

}  // namespace

int main() {
    // 图形状自述(纯常量导出,不触 GPU;先行核对生成头↔DLL 的调用面通达)。
    const int32_t passes = uc05_graph_pass_count();
    if (passes != 2) {
        std::fprintf(stderr, "UC05_HOST: unexpected pass count %d (expected 2)\n", passes);
        return 1;
    }

    if (cudaSetDevice(0) != cudaSuccess) {
        std::fprintf(stderr, "UC05_HOST: cudaSetDevice(0) failed (no GPU?)\n");
        return 2;
    }

    // 最小 render-graph 上下文:与 CUDA device 同 adapter 的 D3D12 device + queue + fence。
    ComPtr<ID3D12Device> d3d_device;
    ComPtr<ID3D12CommandQueue> d3d_queue;
    if (!create_d3d12_on_cuda_adapter(0, d3d_device, d3d_queue)) {
        std::fprintf(stderr, "UC05_HOST: D3D12 render-graph context unavailable\n");
        return 3;
    }
    ComPtr<ID3D12Fence> fence;
    UINT64 fence_value = 0;
    HANDLE fence_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (fence_event == nullptr) {
        std::fprintf(stderr, "UC05_HOST: CreateEvent failed\n");
        return 4;
    }

    // 负例先行(跨 ABI 状态码面,RD-026 无 Result 面纪律):n 越界 → 状态码 2,**不进 GPU 路**、
    // 不 panic、不跨 ABI 展开。证错误面在 C 边界上是可判定的返回值而非未定义行为。
    int32_t sink = -1;
    const int32_t bad_rc = uc05_run_graph(&sink, 0);
    if (bad_rc != 2) {
        std::fprintf(stderr, "UC05_HOST: expected status 2 for n=0, got %d\n", bad_rc);
        CloseHandle(fence_event);
        return 5;
    }

    // 两个规模:n=256(单块)与 n=1024(grid=4,跨块尾界)。每例夹在宿主 fence 锚点之间。
    const int32_t cases[2] = {256, 1024};
    int32_t sums[2] = {0, 0};
    uint32_t refs[2] = {0, 0};
    for (int c = 0; c < 2; ++c) {
        if (!queue_fence_barrier(d3d_device.Get(), d3d_queue.Get(), fence, fence_value, fence_event)) {
            std::fprintf(stderr, "UC05_HOST: pre-node fence barrier failed\n");
            CloseHandle(fence_event);
            return 6;
        }
        // ── 图节点:Rurix RHI 两 pass compute graph(经 export(c) C ABI)────────────────
        int32_t sum = 0;
        const int32_t rc = uc05_run_graph(&sum, cases[c]);
        if (rc != 0) {
            std::fprintf(stderr, "UC05_HOST: uc05_run_graph(n=%d) rc=%d\n", cases[c], rc);
            CloseHandle(fence_event);
            return 7;
        }
        // ─────────────────────────────────────────────────────────────────────────────
        if (!queue_fence_barrier(d3d_device.Get(), d3d_queue.Get(), fence, fence_value, fence_event)) {
            std::fprintf(stderr, "UC05_HOST: post-node fence barrier failed\n");
            CloseHandle(fence_event);
            return 8;
        }
        sums[c] = sum;
        refs[c] = host_reference(static_cast<uint32_t>(cases[c]));
        if (static_cast<uint32_t>(sum) != refs[c]) {
            std::fprintf(stderr, "UC05_HOST: numeric mismatch n=%d sum=%d ref=%u\n", cases[c], sum,
                         refs[c]);
            CloseHandle(fence_event);
            return 9;
        }
        std::printf("UC05_EMBED_CASE n=%d sum=%d ref=%u\n", cases[c], sum, refs[c]);
    }
    CloseHandle(fence_event);

    // 机器可核对标记(ci/uc05_engine_embed_smoke.py device 段解析)。
    std::printf("UC05_EMBED_OK passes=%d cases=2 sum=%d ref=%u d3d12=true\n", passes, sums[0],
                refs[0]);
    return 0;
}
