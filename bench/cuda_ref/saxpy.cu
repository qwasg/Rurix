// M8.2 手写 CUDA C++ 对照: SAXPY out[i] = a*x[i] + y[i]。
// 算法对齐 src/rurix-rt/kernels/saxpy.rx;C ABI 对齐 rurix-rt saxpy 真跑驱动
// (out/x/y 设备指针 u64,a 按值 float,n 按值 u64;out 与 y 分离写独立缓冲)。
#include <cstdint>

extern "C" __global__ void cuda_saxpy(
    uint64_t out_u, uint64_t x_u, uint64_t y_u, float a, uint64_t n_u)
{
    float* __restrict__ out = reinterpret_cast<float*>(out_u);
    const float* __restrict__ x = reinterpret_cast<const float*>(x_u);
    const float* __restrict__ y = reinterpret_cast<const float*>(y_u);
    const uint64_t n = n_u;

    const uint64_t i = static_cast<uint64_t>(blockIdx.x) * blockDim.x + threadIdx.x;
    if (i < n) {
        out[i] = a * x[i] + y[i];
    }
}
