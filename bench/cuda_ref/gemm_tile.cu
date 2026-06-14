// M5.3 手写 CUDA C++ 对照: 经典 16x16 shared-memory tiled GEMM(不触 Tensor Core)。
// 算法对齐 src/rurix-rt/kernels/gemm_tile.rx;C ABI 对齐 rurix-rt gemm_tile 真跑驱动。
#include <cstdint>

extern "C" __global__ void cuda_gemm_tile(
    uint64_t a_u, uint64_t b_u, uint64_t c_u,
    uint64_t m_u, uint64_t n_u, uint64_t k_u)
{
    const float* __restrict__ a = reinterpret_cast<const float*>(a_u);
    const float* __restrict__ b = reinterpret_cast<const float*>(b_u);
    float* __restrict__ c = reinterpret_cast<float*>(c_u);
    const uint64_t m = m_u;
    const uint64_t n = n_u;
    const uint64_t k_dim = k_u;

    __shared__ float atile[256];
    __shared__ float btile[256];

    const unsigned tx = threadIdx.x;
    const unsigned ty = threadIdx.y;
    const uint64_t row = static_cast<uint64_t>(blockIdx.y) * blockDim.y + ty;
    const uint64_t col = static_cast<uint64_t>(blockIdx.x) * blockDim.x + tx;

    float acc = 0.0f;
    const uint64_t ntiles = (k_dim + 15) / 16;
    for (uint64_t tcur = 0; tcur < ntiles; ++tcur) {
        const uint64_t acol = tcur * 16 + tx;
        atile[ty * 16 + tx] = (row < m)
            ? ((acol < k_dim) ? a[row * k_dim + acol] : 0.0f)
            : 0.0f;
        const uint64_t brow = tcur * 16 + ty;
        btile[ty * 16 + tx] = (brow < k_dim)
            ? ((col < n) ? b[brow * n + col] : 0.0f)
            : 0.0f;
        __syncthreads();
        for (unsigned kk = 0; kk < 16; ++kk) {
            acc += atile[ty * 16 + kk] * btile[kk * 16 + tx];
        }
        __syncthreads();
    }
    if (row < m && col < n) {
        c[row * n + col] = acc;
    }
}
