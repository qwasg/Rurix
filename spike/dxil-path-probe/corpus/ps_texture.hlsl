// SPIKE(RD-010) B 路代表性语料 — pixel / 采样器 + 纹理。
// 测转译层对 Texture2D / SamplerState / SV_Target 资源绑定语义保真。
Texture2D tex : register(t0);
SamplerState samp : register(s0);
struct PSIn {
    float4 pos : SV_Position;
    float2 uv : TEXCOORD0;
};
float4 main(PSIn input) : SV_Target {
    return tex.Sample(samp, input.uv) * float4(1.0f, 0.5f, 0.25f, 1.0f);
}
