// SPIKE(RD-010) B 路代表性语料 — compute / SAXPY 形(RWStructuredBuffer 读改写)。
// 经 dxc -spirv 产 SPIR-V → spirv-cross 回 HLSL → dxc 产 DXIL,端到端转译取证。
RWStructuredBuffer<float> buf : register(u0);
[numthreads(64, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID) {
    buf[tid.x] = buf[tid.x] * 2.0f + 1.0f;
}
