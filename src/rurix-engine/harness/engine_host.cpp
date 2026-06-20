// engine_host.cpp — 自建最小 C++/D3D12 渲染 harness（G1.3，D-G1-3 / G-G1-3，UC-05 前奏；
// spec/engine_integration.md RXS-0149 / MR-0002）。
//
// 宿主 C++/D3D12 框架是驱动方，Rurix DLL 是被调的 compute pass 提供方（与 G1.1「Rust 驱动
// C++ shim」极性相反）。本 harness 建立最小 render-graph 上下文（在与 CUDA device 同 adapter
// = LUID 匹配的 D3D12 device + command queue），把一个 Rurix compute pass（SAXPY，经
// rurix_engine.dll C ABI）作为图节点执行，端到端**数值对照**（vs host 参考）后打印机器可核对
// 标记，供 CI 步骤 43 device 段解析（ci/engine_integration_smoke.py）。
//
// 复用：compute pass 复用 M8.1 既有 C ABI（rurix-interop RXS-0125，语义 0-byte）+ device kernel
// （saxpy，PTX 经 build.rs 嵌入）；设备指针在 device 0 primary context 内分配（cudaMalloc，与
// rurix_rt Context::from_primary(0) 同 context，对齐 UC-01 零拷贝设备指针约定）。
//
// 范围：仅承担 compute pass，不进图形着色阶段 / DXIL（G2，D-131）。present 呈现对照复用 G1.1
// interop 共享 resource 通路（rurix-d3d12，可选扩展）——本最小 harness 以数值对照为机器核对核心。
//
// 编译（device 段，owner 交互桌面 MSVC + Windows SDK + CUDA Toolkit）：
//   cl /std:c++17 /EHsc /I <crate>/include /I "%CUDA_PATH%\include" engine_host.cpp ^
//      /link rurix_engine.dll.lib cudart.lib d3d12.lib dxgi.lib /LIBPATH:"%CUDA_PATH%\lib\x64"
// 运行：engine_host.exe   （退出码 0 = 数值对照通过；非 0 = compute pass / 对照失败）

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <wrl/client.h>

#include <cuda_runtime.h>

#include "rurix_engine.h"

using Microsoft::WRL::ComPtr;

namespace {

constexpr int kN = 4096;
constexpr float kA = 2.0f;

bool create_d3d12_on_cuda_adapter(int cuda_device, ComPtr<ID3D12Device>& device,
                                  ComPtr<ID3D12CommandQueue>& queue) {
    cudaDeviceProp prop{};
    if (cudaGetDeviceProperties(&prop, cuda_device) != cudaSuccess) {
        std::fprintf(stderr, "ENGINE_HOST: cudaGetDeviceProperties failed\n");
        return false;
    }
    ComPtr<IDXGIFactory4> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) {
        std::fprintf(stderr, "ENGINE_HOST: CreateDXGIFactory2 failed\n");
        return false;
    }
    // 选与 CUDA device LUID 相同的 DXGI adapter（RFC-0001 §4.4 LUID 匹配）。
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
        std::fprintf(stderr, "ENGINE_HOST: no DXGI adapter matches CUDA LUID\n");
        return false;
    }
    if (FAILED(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0,
                                 IID_PPV_ARGS(&device)))) {
        std::fprintf(stderr, "ENGINE_HOST: D3D12CreateDevice failed\n");
        return false;
    }
    D3D12_COMMAND_QUEUE_DESC qdesc{};
    qdesc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    if (FAILED(device->CreateCommandQueue(&qdesc, IID_PPV_ARGS(&queue)))) {
        std::fprintf(stderr, "ENGINE_HOST: CreateCommandQueue failed\n");
        return false;
    }
    return true;
}

}  // namespace

int main() {
    // ABI 版本握手（链接前校核，对齐 RFC-0001 §4.2.1）。
    if (rurix_engine_abi_version() != RURIX_ENGINE_ABI_VERSION) {
        std::fprintf(stderr, "ENGINE_HOST: ABI version mismatch\n");
        return 1;
    }

    if (cudaSetDevice(0) != cudaSuccess) {
        std::fprintf(stderr, "ENGINE_HOST: cudaSetDevice(0) failed (no GPU?)\n");
        return 2;
    }

    // 最小 render-graph 上下文：在与 CUDA device 同 adapter 的 D3D12 device + queue 上承载。
    ComPtr<ID3D12Device> d3d_device;
    ComPtr<ID3D12CommandQueue> d3d_queue;
    if (!create_d3d12_on_cuda_adapter(0, d3d_device, d3d_queue)) {
        std::fprintf(stderr, "ENGINE_HOST: D3D12 render-graph context unavailable\n");
        return 3;
    }

    // compute pass 输入：device 0 primary context 设备 buffer（与 rurix_rt from_primary(0) 同 context）。
    std::vector<float> hx(kN), hy(kN), hout(kN, 0.0f);
    for (int i = 0; i < kN; ++i) {
        hx[i] = static_cast<float>(i) * 0.5f;
        hy[i] = static_cast<float>(i) * 0.25f;
    }
    float *d_out = nullptr, *d_x = nullptr, *d_y = nullptr;
    size_t bytes = static_cast<size_t>(kN) * sizeof(float);
    if (cudaMalloc(&d_out, bytes) != cudaSuccess || cudaMalloc(&d_x, bytes) != cudaSuccess ||
        cudaMalloc(&d_y, bytes) != cudaSuccess) {
        std::fprintf(stderr, "ENGINE_HOST: cudaMalloc failed\n");
        return 4;
    }
    cudaMemcpy(d_x, hx.data(), bytes, cudaMemcpyHostToDevice);
    cudaMemcpy(d_y, hy.data(), bytes, cudaMemcpyHostToDevice);

    // render-graph compute 节点：Rurix DLL SAXPY compute pass（C ABI，复用 RXS-0125）。
    int32_t rc = rurix_engine_compute_saxpy(reinterpret_cast<uint64_t>(d_out),
                                            reinterpret_cast<uint64_t>(d_x),
                                            reinterpret_cast<uint64_t>(d_y), kA,
                                            static_cast<uint64_t>(kN));
    if (rc != 0) {
        std::fprintf(stderr, "ENGINE_HOST: rurix_engine_compute_saxpy rc=%d\n", rc);
        return 5;
    }
    cudaMemcpy(hout.data(), d_out, bytes, cudaMemcpyDeviceToHost);

    // 端到端数值对照（out == a*x + y）+ 确定性 checksum（FNV-1a 64）。
    uint64_t checksum = 0xcbf29ce484222325ULL;
    int mismatches = 0;
    for (int i = 0; i < kN; ++i) {
        float expected = kA * hx[i] + hy[i];
        if (std::abs(hout[i] - expected) > 1e-3f * (1.0f + std::abs(expected))) {
            ++mismatches;
        }
        uint32_t bits;
        std::memcpy(&bits, &hout[i], sizeof(bits));
        checksum ^= bits;
        checksum *= 0x100000001b3ULL;
    }
    cudaFree(d_out);
    cudaFree(d_x);
    cudaFree(d_y);

    if (mismatches != 0) {
        std::fprintf(stderr, "ENGINE_HOST: numeric mismatch count=%d\n", mismatches);
        return 6;
    }
    // 机器可核对标记（ci/engine_integration_smoke.py device 段解析）。
    std::printf("ENGINE_INTEGRATION: ok pass=saxpy numeric=ok n=%d checksum=%016llx present=false\n",
                kN, static_cast<unsigned long long>(checksum));
    return 0;
}
