// M5.3 手写 CUDA C++ 对照: block 级 shared 树形归约(atomics-free)。
// 算法对齐 src/rurix-rt/kernels/reduce.rx;C ABI 对齐 rurix-rt reduce 真跑驱动。
#include <cstdint>

extern "C" __global__ void cuda_reduce(
    uint64_t src_u, uint64_t partials_u, uint64_t n_u)
{
    const float* __restrict__ src = reinterpret_cast<const float*>(src_u);
    float* __restrict__ partials = reinterpret_cast<float*>(partials_u);
    const uint64_t n = n_u;

    __shared__ float tile[256];
    const unsigned tid = threadIdx.x;
    const uint64_t i = static_cast<uint64_t>(blockIdx.x) * blockDim.x + tid;

    tile[tid] = (i < n) ? src[i] : 0.0f;
    __syncthreads();

    unsigned stride = blockDim.x / 2;
    while (stride > 0) {
        if (tid < stride) {
            tile[tid] += tile[tid + stride];
        }
        __syncthreads();
        stride /= 2;
    }
    if (tid == 0) {
        partials[blockIdx.x] = tile[0];
    }
}
