// M5.3 手写 CUDA C++ 对照: block-local Hillis-Steele inclusive scan。
// 算法对齐 src/rurix-rt/kernels/scan.rx;C ABI 对齐 rurix-rt scan 真跑驱动。
#include <cstdint>

extern "C" __global__ void cuda_scan(
    uint64_t src_u, uint64_t dst_u, uint64_t n_u)
{
    const float* __restrict__ src = reinterpret_cast<const float*>(src_u);
    float* __restrict__ dst = reinterpret_cast<float*>(dst_u);
    const uint64_t n = n_u;

    __shared__ float buf[256];
    const unsigned tid = threadIdx.x;
    const uint64_t i = static_cast<uint64_t>(blockIdx.x) * blockDim.x + tid;

    buf[tid] = (i < n) ? src[i] : 0.0f;
    __syncthreads();

    unsigned offset = 1;
    while (offset < blockDim.x) {
        float v = buf[tid];
        if (tid >= offset) {
            v += buf[tid - offset];
        }
        __syncthreads();
        buf[tid] = v;
        __syncthreads();
        offset *= 2;
    }
    if (i < n) {
        dst[i] = buf[tid];
    }
}
