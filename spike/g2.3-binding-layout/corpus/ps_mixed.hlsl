// SPIKE(G2.3) — 不入 src/ 生产路径，spike 结束可弃。
// 语料②：混合资源种类（cbuffer b / 多 Texture t / 多 Sampler s），用于 Q-Space
// 「按种类分 space 还是按声明序打包」的实测对照。显式 vk::binding 模拟 Rurix
// 按 io_sig 顺序确定性分配 (binding, set)。
[[vk::binding(0, 0)]] cbuffer Globals      { float4 tint; }
[[vk::binding(1, 0)]] Texture2D<float4> albedo;
[[vk::binding(2, 0)]] Texture2D<float4> normal;
[[vk::binding(3, 0)]] SamplerState samp_linear;
[[vk::binding(4, 0)]] SamplerState samp_point;

float4 main(float2 uv : TEXCOORD0) : SV_Target {
    float4 a = albedo.Sample(samp_linear, uv);
    float4 n = normal.Sample(samp_point, uv);
    return (a + n) * tint;
}
