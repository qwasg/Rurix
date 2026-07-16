// SPIKE(RD-010) B 路代表性语料 — compute / groupshared + barrier 归约形。
// 覆盖 shared memory / GroupMemoryBarrierWithGroupSync 语义,测转译层对同步原语保真。
RWStructuredBuffer<float> data : register(u0);
groupshared float scratch[64];
[numthreads(64, 1, 1)]
void main(uint3 gtid : SV_GroupThreadID, uint3 gid : SV_GroupID) {
    uint i = gtid.x;
    scratch[i] = data[gid.x * 64 + i];
    GroupMemoryBarrierWithGroupSync();
    [unroll]
    for (uint s = 32; s > 0; s >>= 1) {
        if (i < s) {
            scratch[i] += scratch[i + s];
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (i == 0) {
        data[gid.x] = scratch[0];
    }
}
