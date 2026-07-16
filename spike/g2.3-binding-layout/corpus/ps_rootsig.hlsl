// SPIKE(G2.3) — 不入 src/ 生产路径，spike 结束可弃。
// 语料④：与 ps_mixed 同资源面，但携显式 [RootSignature] 属性。用于 Q-RootShape
// 实测「dxc 是否把 root signature 序列化进 DXIL 容器（RTS0 part）」——对照语料①~③
// 默认编译（无 root signature 属性）是否产 RTS0。该属性手写 root signature 字符串，
// 模拟 Rurix 推导后可注入的 root signature 形态（CBV root descriptor + descriptor
// table 混合）。本语料仅证「编译器侧 root signature 可序列化进容器」，不证 Rurix 推导。
#define RS "RootFlags(0), " \
           "CBV(b0), " \
           "DescriptorTable(SRV(t0, numDescriptors=2)), " \
           "StaticSampler(s0), StaticSampler(s1)"

cbuffer Globals : register(b0)       { float4 tint; }
Texture2D<float4> albedo : register(t0);
Texture2D<float4> normal : register(t1);
SamplerState samp_linear : register(s0);
SamplerState samp_point  : register(s1);

[RootSignature(RS)]
float4 main(float2 uv : TEXCOORD0) : SV_Target {
    float4 a = albedo.Sample(samp_linear, uv);
    float4 n = normal.Sample(samp_point, uv);
    return (a + n) * tint;
}
