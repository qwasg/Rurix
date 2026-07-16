// SPIKE(G2.3) — 不入 src/ 生产路径，spike 结束可弃。
// 语料③：structured buffer（SRV t）+ RWStructuredBuffer（UAV u）+ cbuffer（b）。
// 覆盖 SRV/UAV/CBV 三类 root parameter 候选，用于 Q-RootShape root descriptor vs
// descriptor table 推导实测。compute 阶段（D3D12 语境，RXS-0153 compute-via-kernel）。
[[vk::binding(0, 0)]] cbuffer Params            { uint count; }
[[vk::binding(1, 0)]] StructuredBuffer<float>   src;
[[vk::binding(2, 0)]] RWStructuredBuffer<float> dst;

[numthreads(64, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID) {
    if (tid.x < count) {
        dst[tid.x] = src[tid.x] * 2.0;
    }
}
