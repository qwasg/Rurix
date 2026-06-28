// SPIKE(RD-010-adjacent / G2.3) — 不入 src/ 生产路径，spike 结束可弃。
// 语料①：RXS-0156 核心面 = Texture2D<F> + Sampler（最小绑定面）。
// 显式 vk::binding 控制 SPIR-V (binding, set)，模拟 Rurix 按 io_sig 顺序确定性分配。
// binding 0 set 0 = SRV 纹理；binding 0 set 0 = sampler（HLSL register class 区分 t/s）。
[[vk::binding(0, 0)]] Texture2D<float4> tex0;
[[vk::binding(1, 0)]] SamplerState samp0;

float4 main(float2 uv : TEXCOORD0) : SV_Target {
    return tex0.Sample(samp0, uv);
}
