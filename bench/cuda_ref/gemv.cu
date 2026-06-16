// M8.2 手写 CUDA C++ 对照: GEMV y[M] = A[M,N]·x[N](行主序,访存受限)。
// cublas GEMV 三层绑定(rurix-cublas)的 ≥90% 性能对照基线(01 §6 UC-01 判据)。
// 每线程算一输出行(row),沿 N 维点积;A 行主序(lda=N),x/y 连续。
// C ABI:y/A/x 设备指针 u64,m(=A 行数=y 长度)/n(=A 列数=x 长度)按值 u64。
#include <cstdint>

extern "C" __global__ void cuda_gemv(
    uint64_t y_u, uint64_t a_u, uint64_t x_u, uint64_t m_u, uint64_t n_u)
{
    float* __restrict__ y = reinterpret_cast<float*>(y_u);
    const float* __restrict__ a = reinterpret_cast<const float*>(a_u);
    const float* __restrict__ x = reinterpret_cast<const float*>(x_u);
    const uint64_t m = m_u, n = n_u;

    const uint64_t row = static_cast<uint64_t>(blockIdx.x) * blockDim.x + threadIdx.x;
    if (row < m) {
        float acc = 0.0f;
        const uint64_t base = row * n;
        for (uint64_t j = 0; j < n; ++j) {
            acc += a[base + j] * x[j];
        }
        y[row] = acc;
    }
}
