// SPIKE(RD-010) B 路代表性语料 — vertex / 带 IO 语义(position + uv)。
// 测转译层对 stage IO(SV_Position / TEXCOORD)与矩阵乘的保真。
struct VSIn {
    float3 pos : POSITION;
    float2 uv : TEXCOORD0;
};
struct VSOut {
    float4 pos : SV_Position;
    float2 uv : TEXCOORD0;
};
cbuffer Camera : register(b0) {
    float4x4 mvp;
};
VSOut main(VSIn input) {
    VSOut o;
    o.pos = mul(mvp, float4(input.pos, 1.0f));
    o.uv = input.uv;
    return o;
}
